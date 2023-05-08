#![doc = include_str!("../README.md")]

mod adapters;
mod driver;
mod list;
mod methods;
mod set;

pub use driver::Driver;
pub use methods::RawMethod;
pub use set::*;

use anyhow::Result;
use fancy_regex::Regex;
use rhai::{Dynamic, Engine, EvalAltResult};

/// A Forne engine, which can act as the backend for learn operations. An instance of this `struct` should be
/// instantiated with a [`Set`] to operate on and an operation to perform.
///
/// The engine has the same lifetime as the reference it is given to its interface for communicating with the host
/// environment.
pub struct Forne {
    /// The set being operated on.
    set: Set,
    /// A Rhai scripting engine used to compile and execute the scripts that drive adapters and learning methods.
    rhai_engine: Engine,
}
impl Forne {
    /// Creates a new set from the given source file text and adapter script. This is a thin wrapper over the `Set::new_with_adapter`
    /// method, abstracting away the internal use of a Rhai engine. In general, you should prefer this method, as there is no additional
    /// overhead to using it.
    pub fn new_set(src: String, adapter_script: &str, raw_method: RawMethod) -> Result<Self> {
        let engine = Self::create_engine();
        let set = Set::new_with_adapter(src, adapter_script, raw_method, &engine)?;

        Ok(Self {
            set,
            rhai_engine: engine,
        })
    }
    /// Creates a new Forne engine. While not inherently expensive, this should generally only be called once, or when
    /// the system needs to restart.
    pub fn from_set(set: Set) -> Self {
        Self {
            set,
            rhai_engine: Self::create_engine(),
        }
    }
    /// Start a new learning session with this instance and the given method (see [`RawMethod`]), creating a [`Driver`]
    /// to run it.
    ///
    /// # Errors
    ///
    /// This will return an error if the given method has not previously been used with this set, and a reset must be performed in that case,
    /// which will lead to the loss of previous progress, unless a transformer is used.
    pub fn learn(&mut self, raw_method: RawMethod) -> Result<Driver<'_, '_>> {
        let driver = Driver::new_learn(&mut self.set, raw_method, &self.rhai_engine)?;
        Ok(driver)
    }
    /// Start a new test with this instance, creating a [`Driver`] to run it.
    pub fn test(&mut self) -> Driver<'_, '_> {
        Driver::new_test(&mut self.set)
    }
    /// Saves this set to JSON.
    ///
    /// # Errors
    ///
    /// This can only possible fail if the learning method produces metadata that cannot be serialized into JSON.
    // TODO Is that even possible with Rhai objects?
    pub fn save_set(&self) -> Result<String> {
        self.set.save()
    }
    /// Resets all cards in a learn session back to the default metadata values prescribed by the learning method.
    pub fn reset_learn(&mut self, method: RawMethod) -> Result<()> {
        let method = method.into_method(&self.rhai_engine)?;
        self.set.reset_learn((method.get_default_metadata)()?);

        Ok(())
    }
    /// Resets all test progress for this set. This is irreversible!
    ///
    /// This will not change whether or not cards are starred.
    pub fn reset_test(&mut self) {
        self.set.reset_test();
    }

    /// Creates a Rhai engine with the utilities Forne provides all pre-registered.
    fn create_engine() -> Engine {
        let mut engine = Engine::new();
        // Regex utilities (with support for backreferences etc.)
        engine.register_fn("is_match", |regex: String, text: String| {
            let re = Regex::new(&regex).map_err(|e| e.to_string())?;
            let is_match = re.is_match(&text).map_err(|e| e.to_string())?;
            Ok::<_, Box<EvalAltResult>>(Dynamic::from_bool(is_match))
        });
        engine.register_fn("matches", |regex: &str, text: &str| {
            let re = Regex::new(regex).map_err(|e| e.to_string())?;
            let mut matches = Vec::new();
            for m in re.find_iter(text) {
                let m = m.map_err(|e| e.to_string())?.as_str();
                matches.push(Dynamic::from(m.to_string()));
            }
            Ok::<_, Box<EvalAltResult>>(Dynamic::from_array(matches))
        });
        engine.register_fn("captures", |regex: &str, text: &str| {
            let re = Regex::new(regex).map_err(|e| e.to_string())?;
            let mut capture_groups = Vec::new();
            for raw_caps in re.captures_iter(text) {
                let raw_caps = raw_caps.map_err(|e| e.to_string())?;
                let mut caps = Vec::new();
                for cap in raw_caps.iter() {
                    let cap = cap.ok_or("invalid capture found")?.as_str();
                    caps.push(Dynamic::from(cap.to_string()));
                }
                capture_groups.push(Dynamic::from_array(caps));
            }

            Ok::<_, Box<EvalAltResult>>(Dynamic::from_array(capture_groups))
        });
        engine.register_fn(
            "replace_one",
            |regex: &str, replacement: &str, text: &str| {
                let re = Regex::new(regex).map_err(|e| e.to_string())?;
                let result = re.replace(text, replacement).into_owned();
                Ok::<_, Box<EvalAltResult>>(Dynamic::from(result))
            },
        );
        engine.register_fn(
            "replace_all",
            |regex: &str, replacement: &str, text: &str| {
                let re = Regex::new(regex).map_err(|e| e.to_string())?;
                let result = re.replace_all(text, replacement).into_owned();
                Ok::<_, Box<EvalAltResult>>(Dynamic::from(result))
            },
        );
        engine.register_fn(
            "regexp_to_pairs",
            |regex: &str, question_idx: i64, answer_idx: i64, text: &str| {
                let re = Regex::new(regex).map_err(|e| e.to_string())?;
                let mut pairs = Vec::new();
                for raw_caps in re.captures_iter(text) {
                    let raw_caps = raw_caps.map_err(|e| e.to_string())?;
                    let question = raw_caps
                        .get(question_idx as usize)
                        .ok_or("question index did not exist (did you start from 1?)")?
                        .as_str();
                    let answer = raw_caps
                        .get(answer_idx as usize)
                        .ok_or("answer index did not exist (did you start from 1?)")?
                        .as_str();

                    pairs.push(Dynamic::from_array(vec![question.into(), answer.into()]));
                }

                Ok::<_, Box<EvalAltResult>>(Dynamic::from_array(pairs))
            },
        );

        engine
    }
}
