use fancy_regex::Regex;
use anyhow::{Result, Error, bail};
use serde::{Serialize, Deserialize};
use std::io::{self, Write};
use std::collections::HashMap;
use rand::{seq::SliceRandom, distributions::weighted::WeightedError};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use lazy_static::lazy_static;

lazy_static! {
    static ref METHODS: HashMap<String, Method> = {
        let mut map = HashMap::new();
        // Speed v1
        map.insert("speed-1".to_string(), Method {
            responses: vec![
                "y".to_string(),
                "n".to_string(),
            ],
            get_weight: Box::new(|card| {
                // We don't care about difficult and starred cards for now
                card.weight
            }),
            adjust_weight: Box::new(|res, card| {
                if res == "y" && card.weight > 1.0 {
                    // If the user got the card wrong before, reset it now
                    card.weight = 1.0
                } else if res == "y" {
                    // Weight starts at 1.0, so two correct answers will eliminate the card
                    card.weight -= 0.5
                } else if res == "n" {
                    card.weight *= 2.0
                } else {
                    unreachable!()
                }
            })
        });
        // TODO More methods!

        // The special method for tests
        map.insert("test".to_string(), Method {
            responses: vec![
                "y".to_string(),
                "n".to_string(),
            ],
            get_weight: Box::new(|card| {
                if card.seen_in_test {
                    0.0
                } else if card.starred {
                    // Give starred cards slightly higher weights relatively
                    1.5
                } else {
                    1.0
                }
            }),
            // Test results never change weightings, just starrings
            adjust_weight: Box::new(|res, card| {
                if res == "y" {
                    card.starred = false;
                } else if res == "n" {
                    card.starred = true;
                } else {
                    unreachable!()
                }

                card.seen_in_test = true;
            })
        });

        map
    };
}

fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<String>>();
    let op = match args.get(1) {
        Some(op) => op,
        None => bail!("you must provide an operation to perform"),
    };
    if op == "create" {
        let filename = match args.get(2) {
            Some(f) => f,
            None => bail!("you must provide a filename to create the set from"),
        };
        let output = match args.get(3) {
            Some(o) => o,
            None => bail!("you must provide an output file to output this set to"),
        };

        let set = Set::from_org(&filename)?;
        set.save_to_json(output)?;
    } else if op == "run" {
        let filename = match args.get(2) {
            Some(f) => f,
            None => bail!("you must provide a filename to create the set from"),
        };
        let method = match args.get(3) {
            Some(m) => m,
            None => bail!("you must provide a run method to use"),
        };
        // If provided, limit the number of terms studied in any one go to a count
        let count: Option<u32> = args.get(4).map(|x| x.parse().unwrap());
        let mut set = Set::from_json(&filename)?;

        // Invoke the command loop, but save the set before propagating errors
        let res = command_loop(&mut set, method, count);
        set.save_to_json(&filename)?;
        println!("Set saved.");
        res?;

    } else if op == "methods" {
        for (idx, method) in METHODS.keys().enumerate() {
            println!("{}. {}", idx + 1, method);
        }
    } else {
        bail!("invalid operation");
    }

    println!("Goodbye!");
    Ok(())
}

fn command_loop(set: &mut Set, method: &str, count: Option<u32>) -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    loop {
        let mut input = String::new();
        print!("> ");
        stdout.flush()?;
        let read_res = stdin.read_line(&mut input);
        match read_res {
            Ok(n) if n == 0 => {
                println!("\n");
                break;
            },
            Ok(_) => {
                parse_command(&input, method, set, count)?;
            },
            Err(_) => bail!("failed to read from stdin"),
        }
    }

    Ok(())
}

/// Parses the given command.
fn parse_command(command: &str, method: &str, set: &mut Set, count: Option<u32>) -> Result<()> {
    let command = command.strip_suffix("\n").unwrap_or(command);
    if command == "learn" {
        set.run(method, RunTarget::All, count)?;
    } else if command == "learn starred" {
        set.run(method, RunTarget::Starred, count)?;
    } else if command == "learn difficult" {
        set.run(method, RunTarget::Difficult, count)?;
    } else if command == "test" {
        set.run("test", RunTarget::All, count)?;
    } else if command == "test starred" {
        set.run("test", RunTarget::Starred, count)?;
    } else if command == "test difficult" {
        set.run("test", RunTarget::Difficult, count)?;
    } else if command == "reset stars" {
        set.reset_stars();
    } else if command == "reset ALL" {
        // Highly destructive!
        set.reset_run();
        set.reset_test();
        set.reset_stars();
    } else {
        println!("Invalid command!");
    }

    Ok(())
}

/// A set of cards with associated data about how learning this set has progressed.
#[derive(Serialize, Deserialize)]
struct Set {
    cards: Vec<Card>,
    /// The state of the set in terms of tests. This will be `Some(..)` if there was a previous
    /// test, and the attached string will be the name of the method used. Runs on different targets
    /// will not interfere with each other, and this program is built to support them.
    run_state: Option<String>,
    test_in_progress: bool,
}
impl Set {
    /// Initiates a runthrough of this set with the given method name and target.
    ///
    /// When the method name is `test`, the user is merely asked if they know each card, regardless of
    /// the weight previously assigned to it, and it will be starred if necessary. Tests do
    /// NOT alter learning weights at all.
    fn run(&mut self, method_name: &str, target: RunTarget, count: Option<u32>) -> Result<()> {
        let method_name = method_name.to_string(); // Matches
        let method = match METHODS.get(&method_name) {
            Some(method) => method,
            None => bail!("invalid method!")
        };
        let mut rng = rand::thread_rng();

        // Check if we should use the previous run's progress (we'll ask the user, and really check if we
        // used a different method last time)
        //
        // If this is a test though, we don't need to do any of this
        let use_previous = if method_name == "test" {
            // If there's a test in progress and we aren't continuing, reset
            // If there isn't, still reset to clean up
            if !self.test_in_progress || self.test_in_progress && !confirm("Would you like to continue your previous test?")? {
                self.reset_test();
            }
            // We don't care about weights in a test
            true
        } else if self.run_state == Some(method_name.clone()) && confirm("Would you like to continue your previous run?")? {
            true
        } else if self.run_state.is_some() && !confirm("Are you sure you want to being a run with this method? There is a previous run in progress with a different method.")? {
            return Ok(());
        } else {
            // 1. We don't want to continue the previous run
            // 2. There was no previous run (probably need to reset weights)
            // 3. The user wants to bulldoze through with a different method
            false
        };
        if !use_previous {
            self.reset_run();
        }

        let mut yellow = ColorSpec::new();
        yellow.set_fg(Some(Color::Yellow));
        let mut green = ColorSpec::new();
        green.set_fg(Some(Color::Green));

        if method_name == "test" {
            self.test_in_progress = true;
        } else {
            self.run_state = Some(method_name.clone());
        }
        let stdin = io::stdin();
        let mut stdout = StandardStream::stdout(ColorChoice::Always);
        for _ in 0..count.unwrap_or(u32::MAX) {
            // Randomly select a card based on the above weights
            let card = match self.cards.choose_weighted_mut(&mut rng, |card: &Card| {
                match target {
                    RunTarget::All => (method.get_weight)(card),
                    RunTarget::Starred if card.starred => (method.get_weight)(card),
                    RunTarget::Difficult if card.difficult => (method.get_weight)(card),
                    _ => 0.0
                }
            }) {
                Ok(card) => card,
                // We're done!
                Err(WeightedError::AllWeightsZero) => {
                    // If we've genuinely finished, say so (but tests will never finish a set in this way)
                    if method_name == "test" {
                        self.test_in_progress = false;
                    } else {
                        self.run_state = None;
                    }
                    break;
                },
                Err(err) => return Err(Error::new(err)),
            };
            stdout.set_color(&yellow)?;
            print!("{}Q: {}", if card.starred {
                "⦿ "
            } else { "" }, card.question);
            stdout.flush()?;
            // Wait for the user to press enter
            let res = stdin.read_line(&mut String::new());
            // If the user wants to end the run, let them (their progress is saved)
            if let Ok(0) = res {
                break;
            }

            stdout.set_color(&green)?;
            println!("A: {}", card.answer);
            stdout.reset()?;

            // Prompt the user for a response based on the method (or y/n if this is a test)
            let res = loop {
                print!(
                    "How did you do? [{}] ",
                    method.responses.join("/"),
                );
                stdout.flush()?;
                let mut input = String::new();
                match stdin.read_line(&mut input) {
                    Ok(_) => {
                        let input = input.strip_suffix("\n").unwrap_or(input.as_str());
                        if method.responses.iter().any(|x| x == input) {
                            break input.to_string();
                        } else {
                            println!("Invalid option!");
                            continue;
                        }
                    }
                    Err(_) => bail!("failed to read from stdin"),
                };
            };
            // The method will decide what to do with that
            (method.adjust_weight)(&res, card);

            println!("---");
        }
        stdout.reset()?;
        println!();

        Ok(())
    }
    /// Resets all run progress for this set. This is irreversible!
    ///
    /// This will not change whether or not cards are starred.
    fn reset_run(&mut self) {
        for card in self.cards.iter_mut() {
            card.weight = 1.0;
        }
    }
    /// Resets all test progress for this set. This is irreversible!
    ///
    /// This will not change whether or not cards are starred.
    fn reset_test(&mut self) {
        for card in self.cards.iter_mut() {
            card.seen_in_test = false;
        }
    }
    /// Resets all stars for this set. This is irreversible!
    fn reset_stars(&mut self) {
        for card in self.cards.iter_mut() {
            card.starred = false;
        }
    }
    /// Creates a new set from the given file of ARQs.
    fn from_org(filename: &str) -> Result<Self> {
        let contents = std::fs::read_to_string(filename)?;

        // Get the question/answer pairs using regexp wizardry
        let re = Regex::new(r#"\*+ \[ \] (.*) :drill:[\s\S]*?(\*+)\* Answer\n([\s\S]*?)(?=(\n\*(?!\2)|$))"#).unwrap();
        let mut cards = Vec::new();
        for caps in re.captures_iter(&contents) {
            let caps = caps?;
            let question = caps.get(1).unwrap().as_str();
            let answer = caps.get(3).unwrap().as_str();
            // Normalise headings out of the answer to make it nicer for simple flashcards
            let answer = Regex::new(r#"(?m)^\*+ "#)
                .unwrap()
                .replace_all(&answer, "");

            let card = Card {
                question: question.to_string(),
                answer: answer.to_string(),
                // Start everything equally
                weight: 1.0,
                starred: false,
                difficult: false,
                seen_in_test: false,
            };
            cards.push(card);
        }

        Ok(Self {
            cards,
            run_state: None,
            test_in_progress: false,
        })
    }
    /// Saves this set to the given JSON file, preserving all progress.
    fn save_to_json(&self, output: &str) -> Result<()> {
        let json = serde_json::to_string(&self)?;
        std::fs::write(output, json)?;
        Ok(())
    }
    /// Loads this set from the given JSON file.
    fn from_json(filename: &str) -> Result<Self> {
        let contents = std::fs::read_to_string(filename)?;
        let set = serde_json::from_str(&contents)?;
        Ok(set)
    }
}

/// Asks the user to confirm something with the given message.
fn confirm(message: &str) -> Result<bool> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    print!("{} [y/n] ", message);
    stdout.flush()?;
    let mut input = String::new();
    let res = match stdin.read_line(&mut input) {
        Ok(_) => {
            let input = input.strip_suffix("\n").unwrap_or(&input);
            if input == "y" {
                true
            } else if input == "n" {
                false
            } else {
                println!("Invalid option!");
                confirm(message)?
            }
        }
        Err(_) => bail!("failed to read from stdin"),
    };

    Ok(res)
}

/// A single key-value pair that represents an element in the set.
#[derive(Serialize, Deserialize)]
struct Card {
    /// The prompt the user will be given for this card.
    question: String,
    /// The answer this card has (which will be shown to the user).
    answer: String,
    /// Whether or not this card has been seen yet in a test.
    seen_in_test: bool,
    /// The weight of this card in the run process, which is a floating-point
    /// number representing the probability that this card will be shown to the user
    /// next (when all those probabilities are summed together). This allows manipulation
    /// by generic learning algorithms.
    weight: f32,
    /// Whether or not this card has been marked as difficult. Difficult cards are intended to
    /// be identified during the learning process, and the marking of them as such should be
    /// automated.
    difficult: bool,
    /// Whether or not this card has been starred.
    starred: bool,
}

/// The different card categories a run might operate on. Since one run's progress will contaminate
/// that o
enum RunTarget {
    All,
    Difficult,
    Starred,
}

/// A method of learning/testing. This program is deliberately as generic as possible, providing
/// a generic execution method for running through a set, which one of these methods can control.
struct Method {
    /// A list of responses the user can give after having been shown the answer to a card. These will
    /// be displayed as options in the order they are provided in here.
    responses: Vec<String>,
    /// A closure that, given a card, produces a weight. This weight represents how
    /// likely the card is to be presented to the user in the next random choice. When a card is finished
    /// with, this should be set to 0.0. When all cards have a weight 0.0, the run will naturally terminate.
    ///
    /// Any cards not part of the relevant run target will not be presented to this function in the first
    /// place.
    get_weight: Box<dyn Fn(&Card) -> f32 + Send + Sync + 'static>,
    /// A closure that, given a card, adjusts the weight for the given card based on
    /// the user's response, which is guaranteed to be one of the provided possible responses.
    adjust_weight: Box<dyn Fn(&str, &mut Card) + Send + Sync + 'static>,
}
