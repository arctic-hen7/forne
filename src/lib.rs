mod methods;
mod adapters;
mod set;
mod list;
mod driver;

pub use driver::Driver;
pub use set::*;
pub use methods::RawMethod;

use rhai::Engine;
use anyhow::Result;

/// A California engine, which can act as the backend for learn operations. An instance of this `struct` should be
/// instantiated with a [`Set`] to operate on and an operation to perform.
///
/// The engine has the same lifetime as the reference it is given to its interface for communicating with the host
/// environment.
pub struct California {
    /// The set being operated on.
    set: Set,
    /// A Rhai scripting engine used to compile and execute the scripts that drive adapters and learning methods.
    rhai_engine: Engine,
}
impl California {
    /// Creates a new California engine. While not inherently expensive, this should generally only be called once, or when
    /// the system needs to restart.
    pub fn new(set: Set) -> Self {
        Self {
            set,
            rhai_engine: Engine::new(),
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
}
