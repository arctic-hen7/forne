use std::collections::HashMap;

use rhai::Dynamic;
use serde::{Serialize, Deserialize};
use anyhow::Result;
use uuid::Uuid;

/// A single key-value pair that represents an element in the set.
#[derive(Serialize, Deserialize, Clone)] // Only internal cloning
pub struct Card {
    /// The prompt the user will be given for this card.
    pub question: String,
    /// The answer this card has (which will be shown to the user).
    pub answer: String,
    /// Whether or not this card has been seen yet in the active test.
    pub seen_in_test: bool,
    /// Whether or not this card has been marked as difficult. Difficult cards are intended to
    /// be identified during the learning process, and the marking of them as such should be
    /// automated.
    pub difficult: bool,
    /// Whether or not this card has been starred. Cards are automatically starred if a user gets
    /// them wrong in a test, and they will be unstarred if the user later gets them right in a test. This
    /// behaviour can be customised with flags.
    pub starred: bool,
    /// Data about this card stored by the current method. This can be serialized and deserialized, but
    /// is completely arbitrary, and different cards may store completely different data here. This should
    /// be passed to and from method scripts with no intervention from Rust.
    pub method_data: Dynamic,
}

/// A slim representation of a card without internal metadata, which will be returned when polling a
/// [`crate::Driver`].
#[derive(Clone)]
pub struct SlimCard {
    /// The question on the card.
    pub question: String,
    /// The answer on the 'other side' of the card.
    pub answer: String,
    /// Whether or not the card has been automatically marked as difficult. Callers may wish to highlight this
    /// to users when a question is displayed, or not.
    pub difficult: bool,
    /// Whether or not the card has been starred, which, likewise, callers may wish to highlight or not when
    /// displaying this card.
    pub starred: bool,
}

/// The different card categories that operations on sets can be classed into.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
pub enum CardType {
    /// All the cards in the set.
    All,
    /// Only cards that have been automatically marked as difficult.
    Difficult,
    /// Only cards that have been automatically starred when the user got them wrong in a test.
    Starred,
}

/// A set of cards with associated data about how learning this set has progressed.
#[derive(Serialize, Deserialize)]
pub struct Set {
    /// The name of the method used on this set. As methods provide their own custom metadata for each card, it
    /// is not generally possible to transition a set from one learning method to another while keeping your
    /// progress, unless a transformer is provided by the methods to do so. This acts as a guard to prevent
    /// the user from accidentally deleting all their hard work!
    pub method: String,
    /// A list of all the cards in the set.
    pub cards: HashMap<Uuid, Card>,
    /// The state of the set in terms of tests. This will be `Some(..)` if there was a previous
    /// test, and the attached string will be the name of the method used. Runs on different targets
    /// will not interfere with each other, and this program is built to support them.
    pub run_state: Option<String>,
    /// Whether or not there is a test currently in progress. Card weightings are calculated with an
    /// internal system in tests, but no internal card metadata will be modified, this is instead used to keep
    /// track of which cards have already been shown to the user.
    ///
    /// Note that, if a test is started on one target, and a later test is begun on a different subset target,
    /// it is possible that the latter will cause the prior to be forgotten about (since this will be set back
    /// to `false` once the active test is finished). This kind of issue does not affect learn mode, because there
    /// is no such thing as a finished learn mode, until all weightings are set to zero, meaning things are kept
    /// track of on a card-by-card basis, unlike in tests.
    pub test_in_progress: bool,
}
impl Set {
    /// Saves this set to the given JSON file, preserving all progress.
    pub fn save(&self) -> Result<String> {
        let json = serde_json::to_string(&self)?;
        Ok(json)
    }
    /// Loads this set from the given JSON.
    pub fn from_json(json: &str) -> Result<Self> {
        let set = serde_json::from_str(&json)?;
        Ok(set)
    }
    /// Resets all test progress for this set. This is irreversible!
    ///
    /// This will not change whether or not cards are starred.
    pub fn reset_test(&mut self) {
        for card in self.cards.values_mut() {
            card.seen_in_test = false;
        }
    }
    /// Resets all stars for this set. This is irreversible!
    pub fn reset_stars(&mut self) {
        for card in self.cards.values_mut() {
            card.starred = false;
        }
    }
}
