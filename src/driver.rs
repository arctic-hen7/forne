use crate::{
    methods::{Method, RawMethod},
    set::{Card, CardType, Set, SlimCard},
};
use anyhow::{bail, Error, Result};
use lazy_static::lazy_static;
use rand::{distributions::WeightedError, seq::SliceRandom};
use rhai::Engine;
use uuid::Uuid;

lazy_static! {
    // Fight me.
    static ref TEST_RESPONSES: &'static [String] = Box::leak(Box::new(["y".to_string(), "n".to_string()]));
}

/// A system to drive user interactions forward by providing a re-entrant polling architecture. The caller should call `.next()`
/// to get the next question/answer pair, providing the user's response for the previous question so the driver can update the set
/// as necessary. This architecture allows the caller much greater control over the order of execution and display than an interface
/// type opaquely driven by this library would.
pub struct Driver<'e, 's> {
    /// The learning method used. This is responsible for the majority of the logic.
    ///
    /// If this is `None`, the driver will run a test instead of a learning session, which uses custom and internal logic.
    method: Option<Method<'e>>,
    /// A mutable reference to the set we're operating on.
    set: &'s mut Set,
    /// The unique identifier of the last card returned by  `.next()` or `.first()`.
    // We can't store a mutable reference to the latest card directly here, because the lifetimes wouldn't work out at all, and this
    // is a really subtle bug that Rust picks up on uniquely and superbly!
    latest_card: Option<Uuid>,
    /// The maximum number of elements to review, if one has been set.
    max_count: Option<u32>,
    /// The number of cards we've reviewed so far.
    curr_count: u32,
    /// The type of cards to be targeted by this driver.
    target: CardType,

    /// Whether or not we should mark cards that the user gets wrong as starred in tests.
    mark_starred: bool,
    /// Whether or not the learning method should be allowed to change the difficulty status of cards.
    mutate_difficulty: bool,
    /// Whether or not we should mark cards that the user gets right in tests as unstarred.
    ///
    /// This is especially useful when there are a small number of cards that the user is getting wrong consistently, which
    /// they want to continue keeping track of, while also still reviewing them many times.
    mark_unstarred: bool,
}
impl<'e, 's> Driver<'e, 's> {
    /// Creates a new driver with the given set and method, with the latter provided as either the name of an inbuilt method or the body of
    /// a custom Rhai script.
    ///
    /// # Errors
    ///
    /// This will return an error if the given method has not previously been used with this set, and a reset must be performed in that case,
    /// which will lead to the loss of previous progress, unless a transformer is used.
    pub(crate) fn new_learn(
        set: &'s mut Set,
        raw_method: RawMethod,
        engine: &'e Engine,
    ) -> Result<Self> {
        let method = raw_method.into_method(engine)?;
        let instance = Self {
            method: Some(method),
            set,
            max_count: None,
            curr_count: 0,
            target: CardType::All,
            latest_card: None,

            mark_starred: true,
            mutate_difficulty: true,
            mark_unstarred: true,
        };
        if !instance.method_correct() {
            bail!("given method is not the same as the one that has been previously used for this set (please reset the set before attempting to use a new method)");
        }

        Ok(instance)
    }
    /// Creates a new driver with the given set, running in test mode. This takes no custom method, as it runs a test, and it is infallible.
    pub(crate) fn new_test(set: &'s mut Set) -> Self {
        Self {
            method: None,
            set,
            max_count: None,
            curr_count: 0,
            target: CardType::All,
            latest_card: None,

            mark_starred: true,
            mutate_difficulty: true,
            mark_unstarred: true,
        }
    }
    /// Sets a specific type of card that this driver will exclusively target. By default, drivers target all cards.
    pub fn set_target(&mut self, target: CardType) -> &mut Self {
        self.target = target;
        self
    }
    /// Sets a maximum number of elements to be reviewed through this driver. This can be useful for long-term learning, in which you only
    /// want to review, say, 30 cards per day.
    ///
    /// Obviously, if there are not enough cards to reach this maximum count, the driver will stop before the count is reached, and will
    /// not go back to the beginning.
    pub fn set_max_count(&mut self, count: u32) -> &mut Self {
        self.max_count = Some(count);
        self
    }
    /// If this driver is being used to run a test, prevents cards the user gets wrong from being automatically starred.
    pub fn no_mark_starred(&mut self) -> &mut Self {
        self.mark_starred = false;
        self
    }
    /// If this driver is being used to run a test, prevents cards the user gets right from being unstarred if they were previously starred.
    ///
    /// This is especially useful when there are a small number of cards that the user is getting wrong consistently, which
    /// they want to continue keeping track of, while also still reviewing them many times.
    pub fn no_mark_unstarred(&mut self) -> &mut Self {
        self.mark_unstarred = false;
        self
    }
    /// If this driver is being used to run a learning session, prevents the learning method from marking cards as difficult, or from downgrading
    /// cards currently marked as difficult to no longer difficult.
    ///
    /// Since the `difficult` metadatum is almost entirely internal, there are generally very few scenarios in which this behaviour is desired.
    pub fn no_mutate_difficulty(&mut self) -> &mut Self {
        self.mutate_difficulty = false;
        self
    }
    /// Gets the number of cards that have been reviewed by this driver so far.
    pub fn get_count(&self) -> u32 {
        self.curr_count
    }
    /// Performs a sanity check that the method this driver has been instantiated with is the same as the one that has been being used for the set.
    fn method_correct(&self) -> bool {
        if let Some(method) = &self.method {
            method.name == self.set.method
        } else {
            // We're running a test
            true
        }
    }
    /// Gets the first question/answer pair of this run. While it is perfectly safe to run this at any time, it
    /// is semantically nonsensical to run this more than once, as California's internals will become completely
    /// useless. If you want to display each card to the user only once, irrespective of the metadata attached to
    /// it, you should instantiate the driver for a test, rather than a learning session.
    ///
    /// This will return `None` if there are no more cards with non-zero weightings, in which case the learn or test
    /// session described by this driver is complete. (If progress is not cleared, this could easily happen with a
    /// `.first()` call.)
    ///
    /// This will automatically continue the most recent session of either learning or testing, if there is one.
    // No instance can be constructed without first checking if the method matches the set, so assuming it does
    // is perfectly safe here.
    pub fn first(&mut self) -> Result<Option<SlimCard>> {
        let mut rng = rand::thread_rng();

        // Update the set's state for either learning or testing
        if let Some(method) = &self.method {
            self.set.run_state = Some(method.name.clone());
        } else {
            self.set.test_in_progress = true;
        }

        if self.max_count.is_some() && self.max_count.unwrap() == self.curr_count {
            return Ok(None);
        }

        // Randomly select a card according to the weights generated by the method
        let mut cards_with_ids = self.set.cards.iter().collect::<Vec<_>>();
        let (card_id, card) =
            match cards_with_ids.choose_weighted_mut(&mut rng, |(_, card): &(&Uuid, &Card)| {
                if let Some(method) = &self.method {
                    let res = match &self.target {
                        CardType::All => {
                            (method.get_weight)(card.method_data.clone(), card.difficult)
                        }
                        CardType::Starred if card.starred => {
                            (method.get_weight)(card.method_data.clone(), card.difficult)
                        }
                        CardType::Difficult if card.difficult => {
                            (method.get_weight)(card.method_data.clone(), card.difficult)
                        }
                        _ => Ok(0.0),
                    };
                    // TODO handle errors (very realistic that they would occur with custom scripts!)
                    res.unwrap()
                } else {
                    match &self.target {
                        CardType::All if !card.seen_in_test => 1.0,
                        CardType::Starred if card.starred && !card.seen_in_test => 1.0,
                        CardType::Difficult if card.difficult && !card.seen_in_test => 1.0,
                        _ => 0.0,
                    }
                }
            }) {
                Ok(data) => data,
                // We're done!
                Err(WeightedError::AllWeightsZero) => {
                    // If we've genuinely finished, say so
                    if let Some(method) = &self.method {
                        self.set.run_state = None;
                        self.set.reset_learn((method.get_default_metadata)()?);
                    } else {
                        self.set.test_in_progress = false;
                        self.set.reset_test();
                    }

                    return Ok(None);
                }
                Err(err) => return Err(Error::new(err)),
            };

        // Using a slim representation avoids potentially expensive cloning of the `Dynamic` data the method
        // maintains about this card
        let slim = SlimCard {
            question: card.question.clone(),
            answer: card.answer.clone(),
            starred: card.starred,
            difficult: card.difficult,
        };

        self.latest_card = Some(**card_id);
        self.curr_count += 1;

        Ok(Some(slim))
    }
    /// Provides the allowed responses for this learn method (or for the test, if this driver is being used for
    /// a test), in the order they were defined. The argument to `.next()` must be an element in the list this
    /// returns.
    pub fn allowed_responses(&self) -> &[String] {
        if let Some(method) = &self.method {
            &method.responses
        } else {
            &TEST_RESPONSES
        }
    }
    /// Gets the next question/answer pair, given a response to the last question/answer. If this is the first,
    /// you should call `.first()` instead, as calling this will lead to an error. Note that the provided response
    /// must be *identical* to one of the responses defined by the method in use (these can be found with `.allowed_responses()`).
    pub fn next(&mut self, response: String) -> Result<Option<SlimCard>> {
        if !self.allowed_responses().iter().any(|x| x == &response) {
            bail!("invalid user response to card");
        }

        if let Some(card_id) = self.latest_card.as_mut() {
            // We know this element exists (we hold the only mutable reference to the set)
            let card = self.set.cards.get_mut(card_id).unwrap();
            if let Some(method) = &self.method {
                let (method_data, difficult) =
                    (method.adjust_card)(response, card.method_data.clone(), card.difficult)?;
                card.method_data = method_data;
                if self.mutate_difficulty {
                    card.difficult = difficult;
                }
            } else {
                card.seen_in_test = true;

                if response == "n" && self.mark_starred {
                    card.starred = true;
                } else if response == "y" && self.mark_unstarred {
                    card.starred = false;
                }

                // Prevent this card from being double-adjusted if there's an error later
                self.latest_card = None;
            }

            // Everything has been adjusted
            self.first()
        } else {
            bail!("called `.next()` before `.first()`, or without handling error");
        }
    }
    /// Saves the underlying set to JSON. This should generally be called between each presentation of a card to ensure the user
    /// does not lose their progress.
    pub fn save_set_to_json(&self) -> Result<String> {
        self.set.save()
    }
}
