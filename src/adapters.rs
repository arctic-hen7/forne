use std::collections::HashMap;

use crate::{set::Set, Card, RawMethod};
use anyhow::{anyhow, Context, Result};
use rhai::{Dynamic, Engine, Scope};
use uuid::Uuid;

impl Set {
    /// Creates a new [`Set`] from the given source using the given Rhai script. The script is required
    /// to assemble a Rhai array of question/answer tuples, and California will do the rest of the work
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
        let method = method.into_method(engine)?;

        let mut scope = Scope::new();
        scope.push_constant("SOURCE", src);
        let raw_array: Vec<Dynamic> = engine
            .eval_with_scope(&mut scope, script)
            .with_context(|| "failed to run adapter script")?;
        let mut cards = HashMap::new();

        for dyn_elem in raw_array {
            let elems: Vec<String> = dyn_elem
                .into_typed_array()
                .map_err(|_| anyhow!("couldn't parse adapter results"))?;

            let card = Card {
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
            cards.insert(Uuid::new_v4(), card);
        }

        Ok(Self {
            method: method.name,
            cards,
            run_state: None,
            test_in_progress: false,
        })
    }
    // /// Updates this set from the given source. This will add any new question/answer pairs the adapter script finds,
    // /// and will update any answers that change. If a question changes, it will be registered as a new card. None of
    // /// the metadata on existing cards will be altered.
    // pub(crate) fn update_with_adapter(&mut self, script: &str, src: String, engine: &Engine) -> Result<()> {
    //     let mut scope = Scope::new();
    //     scope.push_constant("SOURCE", src);
    //     let pairs: Vec<(String, String)> = engine.eval_with_scope(&mut scope, script).with_context(|| "failed to run adapter script")?;
    // }
}
