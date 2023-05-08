// This is the CLI binary, which must be built with the `cli` feature flag
#[cfg(not(feature = "cli"))]
compile_error!("the cli binary must be built with the `cli` feature flag");

#[cfg(feature = "cli")]
fn main() -> anyhow::Result<()> {
    use std::fs;
    use anyhow::Context;
    use clap::Parser;
    use opts::{Args, Command};
    use california::{California, Set};
    use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

    let args = Args::parse();
    match args.command {
        Command::New { input, output, adapter, method } => {
            let contents = fs::read_to_string(input).with_context(|| "failed to read from source file")?;
            let adapter_script = fs::read_to_string(adapter).with_context(|| "failed to read adapter script")?;
            let method = method_from_string(method)?;

            let california = California::new_set(contents, &adapter_script, method)?;
            let json = california.save_set()?;
            fs::write(output, json).with_context(|| "failed to write new set to output file")?;

            println!("New set created!");
        },
        Command::Learn { set: set_file, method, ty, count, reset } => {
            let json = fs::read_to_string(&set_file).with_context(|| "failed to read from set file")?;
            let set = Set::from_json(&json)?;
            let mut california = California::from_set(set);
            let method = method_from_string(method)?;
            if reset && confirm("Are you absolutely certain you want to reset your learn progress? This action is IRREVERSIBLE!!!")? {
                california.reset_learn(method.clone())?;
            } else {
                println!("Continuing with previous progress...");
            }
            let mut driver = california
                .learn(method)?;
            driver.set_target(ty);
            if let Some(count) = count {
                driver.set_max_count(count);
            }

            let num_reviewed = drive(driver, &set_file)?;
            println!("\nLearn session complete! You reviewed {} card(s).", num_reviewed);
        },
        Command::Test { set: set_file, static_test, no_star, no_unstar, ty, count, reset } => {
            let json = fs::read_to_string(&set_file).with_context(|| "failed to read from set file")?;
            let set = Set::from_json(&json)?;
            let mut california = California::from_set(set);
            if reset && confirm("Are you sure you want to reset your test progress?")? {
                california.reset_test();
            } else {
                println!("Continuing with previous progress...");
            }
            let mut driver = california
                .test();
            driver.set_target(ty);
            if let Some(count) = count {
                driver.set_max_count(count);
            }
            if static_test {
                driver.no_mark_starred().no_mark_unstarred();
            } else if no_star {
                driver.no_mark_starred();
            } else if no_unstar {
                driver.no_mark_unstarred();
            }

            let num_reviewed = drive(driver, &set_file)?;
            println!("\nTest complete! You reviewed {} card(s).", num_reviewed);

        },
        Command::List { set, ty } => {
            let json = fs::read_to_string(set).with_context(|| "failed to read from set file")?;
            let set = Set::from_json(&json)?;

            let mut yellow = ColorSpec::new();
            yellow.set_fg(Some(Color::Yellow));
            let mut green = ColorSpec::new();
            green.set_fg(Some(Color::Green));

            let mut stdout = StandardStream::stdout(ColorChoice::Always);
            let mut num_printed = 0;
            let list = set.list(ty);
            for card in list.iter() {
                stdout.set_color(&yellow)?;
                println!("{}Q: {}", if card.starred {
                    "⦿ "
                } else { "" }, card.question);
                stdout.set_color(&green)?;
                println!("A: {}", card.answer);
                stdout.reset()?;

                num_printed += 1;
                // Only print the separator if this isn't the last card
                if list.len() != num_printed {
                    println!("---");
                }
            }
        },
    };

    Ok(())
}

/// Creates a `RawMethod` from a string provided on the command line that might either be the name of an inbuilt method
/// or the path to a custom Rhai script.
///
/// For custom scripts, this will make their name be the filename of the script with the current user's username prefixed.
#[cfg(feature = "cli")]
fn method_from_string(method_str: String) -> anyhow::Result<california::RawMethod> {
    use std::{path::PathBuf, fs};
    use anyhow::bail;
    use california::RawMethod;

    if RawMethod::is_inbuilt(&method_str) {
        Ok(RawMethod::Inbuilt(method_str))
    } else {
        // It's a path to a custom script
        let method_path = PathBuf::from(&method_str);
        if let Ok(contents) = fs::read_to_string(&method_path) {
            // Follow California's recommended naming conventions for custom methods
            let name = format!("{}/{}", whoami::username(), method_path.file_name().unwrap().to_string_lossy());
            Ok(RawMethod::Custom {
                name,
                body: contents
            })
        } else {
            bail!("provided method is not inbuilt and does not represent a valid method file (or if it did, california couldn't read it)")
        }
    }
}

/// Displays questions and answers, receiving input from the user and continuing a learning/testing session. This takes
/// both a driver and the input file that the set is stored in, so it can be periodically saved to prevent lost progress.
///
/// This returns the number of cards reviewed.
#[cfg(feature = "cli")]
fn drive<'a>(mut driver: california::Driver<'a, 'a>, set_file: &str) -> anyhow::Result<u32> {
    use std::{io::{self, Write}, fs};
    use anyhow::{bail, Context};
    use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

    let mut yellow = ColorSpec::new();
    yellow.set_fg(Some(Color::Yellow));
    let mut green = ColorSpec::new();
    green.set_fg(Some(Color::Green));

    let stdin = io::stdin();
    let mut stdout = StandardStream::stdout(ColorChoice::Always);

    let mut card_option = driver.first()?;
    while let Some(card) = card_option {
        // Save the set quickly
        let json = driver.save_set_to_json()?;
        fs::write(set_file, json).with_context(|| "failed to save set to json (progress up to the previous card was saved though)")?;

        stdout.set_color(&yellow)?;
        print!("{}Q: {}", if card.starred {
            "⦿ "
        } else { "" }, card.question);
        stdout.flush()?;
        // Wait for the user to press enter
        let res = stdin.read_line(&mut String::new());
        // If the user wants to end the run, let them (their progress will be saved)
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
                driver.allowed_responses().join("/"),
            );
            stdout.flush()?;
            let mut input = String::new();
            match stdin.read_line(&mut input) {
                Ok(_) => {
                    let input = input.strip_suffix("\n").unwrap_or(input.as_str());
                    if driver.allowed_responses().iter().any(|x| x == input) {
                        break input.to_string();
                    } else {
                        println!("Invalid option!");
                        continue;
                    }
                }
                Err(_) => bail!("failed to read from stdin"),
            };
        };
        // Clear the screen to make sure the user can't cheat
        println!("{}", termion::clear::All);

        // This will adjust weights etc. and get us a new card, if one exists
        card_option = driver.next(res)?;
    }
    stdout.reset()?;

    let json = driver.save_set_to_json()?;
    fs::write(set_file, json).with_context(|| "failed to save set to json (progress up to the previous card was saved though)")?;
    Ok(driver.get_count())
}

/// Asks the user to confirm something with the given message.
#[cfg(feature = "cli")]
fn confirm(message: &str) -> anyhow::Result<bool> {
    use std::io::{self, Write};
    use anyhow::bail;

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

#[cfg(feature = "cli")]
mod opts {
    use std::path::PathBuf;

    use california::CardType;
    use clap::{Parser, Subcommand};

    /// California: a spaced repetition CLI to help you learn stuff
    #[derive(Parser, Debug)]
    #[command(author, version, about, long_about = None)]
    pub struct Args {
        #[clap(subcommand)]
        pub command: Command,
    }

    #[derive(Subcommand, Debug)]
    pub enum Command {
        /// Creates a new set
        New {
            /// The file to create the set from
            input: String,
            /// The file to output the set to as JSON
            output: String,
            /// The path to the adapter script to be used to parse the set
            #[arg(short, long)]
            adapter: PathBuf,
            /// The learning method to use for the new set
            #[arg(short, long)]
            method: String, // Secondary parsing
        },
        /// Starts or resumes a learning session on the given set
        Learn {
            /// The file the set is in
            set: String,
            /// The learning method to use
            #[arg(short, long)]
            method: String, // Secondary parsing
            /// The type of cards to operate on (`all`, `difficult`, or `starred`)
            #[arg(short, long = "type", value_enum, default_value = "all")]
            ty: CardType,
            /// Limit the number of terms studied to the given amount (useful for consistent long-term learning); your progress will be saved
            #[arg(short, long)]
            count: Option<u32>,
            /// Starts a new learn session from scratch, irretrievably deleting any progress in a previous session
            #[arg(long)]
            reset: bool,
        },
        /// Starts or resumes a test on the given set
        Test {
            /// The file the set is in
            set: String,
            /// If set, the test will be made 'static', and will not star terms you get wrong, or unstar terms you
            /// get right (equivalent to `--no-star --no-unstar`)
            #[arg(long = "static")]
            static_test: bool,
            /// Do not mark cards you get wrong as starred
            #[arg(long)]
            no_star: bool,
            /// Do not unstar cards you get right if they're currently starred (useful to review cards without losing which ones you're consistently getting wrong)
            #[arg(long)]
            no_unstar: bool,
            /// The type of cards to operate on (`all`, `difficult`, or `starred`)
            #[arg(short, long = "type", value_enum, default_value = "all")]
            ty: CardType,
            /// Limit the number of terms studied to the given amount (useful for consistent long-term learning); your progress will be saved
            #[arg(short, long)]
            count: Option<u32>,
            /// Starts a new test from scratch, irretrievably deleting any progress in a previous test
            #[arg(long)]
            reset: bool,
        },
        /// Lists all the terms in the given set
        List {
            /// The file the set is in
            set: String,
            /// The type of cards to operate on (`all`, `difficult`, or `starred`)
            #[arg(short, long = "type", value_enum, default_value = "all")]
            ty: CardType,
        },
    }
}


/*
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
*/

// fn _main() -> Result<()> {


    // Ok(())

    // let args = std::env::args().collect::<Vec<String>>();
    // let op = match args.get(1) {
    //     Some(op) => op,
    //     None => bail!("you must provide an operation to perform"),
    // };
    // if op == "create" {
    //     let filename = match args.get(2) {
    //         Some(f) => f,
    //         None => bail!("you must provide a filename to create the set from"),
    //     };
    //     let output = match args.get(3) {
    //         Some(o) => o,
    //         None => bail!("you must provide an output file to output this set to"),
    //     };

    //     let set = Set::from_org(&filename)?;
    //     set.save_to_json(output)?;
    // } else if op == "run" {
    //     let filename = match args.get(2) {
    //         Some(f) => f,
    //         None => bail!("you must provide a filename to create the set from"),
    //     };
    //     let method = match args.get(3) {
    //         Some(m) => m,
    //         None => bail!("you must provide a run method to use"),
    //     };
    //     // If provided, limit the number of terms studied in any one go to a count
    //     let count: Option<u32> = args.get(4).map(|x| x.parse().unwrap());
    //     let mut set = Set::from_json(&filename)?;

    //     // Invoke the command loop, but save the set before propagating errors
    //     let res = command_loop(&mut set, method, count);
    //     set.save_to_json(&filename)?;
    //     println!("Set saved.");
    //     res?;

    // } else if op == "methods" {
    //     for (idx, method) in METHODS.keys().enumerate() {
    //         println!("{}. {}", idx + 1, method);
    //     }
    // } else {
    //     bail!("invalid operation");
    // }

    // println!("Goodbye!");
    // Ok(())
// }
/*
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
}


*/
