use std::collections::HashMap;

use crate::{set::Set, Card, RawMethod};
use anyhow::{anyhow, Context, Result};
use rhai::{Dynamic, Engine, Scope};
use uuid::Uuid;

impl Set {
    /// Creates a new [`Set`] from the given source using the given Rhai script. The script is required
    /// to assemble a Rhai array of question/answer tuples, and Forn will do the rest of the work
    /// to create a full set instance.
    ///
    /// **IMPORTANT:** The engine provided to this function must have the necessary functions registered for
    /// regexp support.
    pub(crate) fn new_with_adapter(
        src: String,
        script: &str,
        method: RawMethod,
        engine: &Engine,
    ) -> Result<Self> {
        // Create an empty set and then populate it
        let mut set = Self {
            method: match &method {
                RawMethod::Inbuilt(name) => name,
                RawMethod::Custom { name, .. } => name,
            }
            .to_string(),
            cards: HashMap::new(),
            run_state: None,
            test_in_progress: false,
        };
        set.update_with_adapter(script, src, method, engine)?;

        Ok(set)
    }
    /// Updates this set from the given source. This will add any new question/answer pairs the adapter script finds,
    /// and will update any answers that change. If a question changes, it will be registered as a new card. Any cards
    /// whose answers change will have their metadata reset in order to allow the user to learn the new card.
    ///
    /// The arguments provided to this function must satisfy the same requirements as those provided to
    /// [`Self::new_with_adapter`].
    pub(crate) fn update_with_adapter(
        &mut self,
        script: &str,
        src: String,
        method: RawMethod,
        engine: &Engine,
    ) -> Result<()> {
        let method = method.into_method(engine)?;

        let mut scope = Scope::new();
        scope.push_constant("SOURCE", src);
        // This will get *all* the cards in the source, which we will then compare
        // with what we already have
        let raw_array: Vec<Dynamic> = engine
            .eval_with_scope(&mut scope, script)
            .with_context(|| "failed to run adapter script")?;

        for dyn_elem in raw_array {
            let elems: Vec<String> = dyn_elem
                .into_typed_array()
                .map_err(|_| anyhow!("couldn't parse adapter results"))?;

            let new_card = Card {
                question: elems
                    .get(0)
                    .ok_or_else(|| anyhow!("adapter did not return question for card"))?
                    .to_string(),
                answer: elems
                    .get(1)
                    .ok_or_else(|| anyhow!("adapter did not return answer for card"))?
                    .to_string(),
                seen_in_test: false,
                difficult: false,
                starred: false,
                method_data: (method.get_default_metadata)()?,
            };
            // If we've already got this question, update the answer if necessary, otherwise add it afresh
            let found = self
                .cards
                .iter_mut()
                .find(|(_id, card)| card.question == new_card.question);
            if let Some((_id, card)) = found {
                *card = new_card;
            } else {
                self.cards.insert(Uuid::new_v4(), new_card);
            }
        }

        Ok(())
    }
}
