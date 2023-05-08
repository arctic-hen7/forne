use anyhow::{anyhow, bail, Context, Result};
use include_dir::{include_dir, Dir};
use rhai::{Array, Dynamic, Engine, Scope, AST};

/// The `src/methods` directory that includes this file.
static METHODS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/methods");

/// A learning method based on closures extracted from a Rhai script.
///
/// Calling the closures this wraps may lead to Rhai script errors, which will be propagated safely.
pub struct Method<'e> {
    /// The name of the method, which will be used by users to specify the learning method they want to use
    /// on the command line: i.e. `--method <name>`. This must not contain spaces, and should be in `kebab-case`.
    pub name: String,
    /// A list of responses the user can give after having been shown the answer to a card. These will
    /// be displayed as options in the order they are provided in here.
    pub responses: Vec<String>,
    /// A closure that, given a card's metadata and whether or not it has been marked as difficult, produces a weight.
    /// This weight represents how likely the card is to be presented to the user in the next random choice. When a card is finished
    /// with, this should be set to 0.0. When all cards have a weight 0.0, the run will naturally terminate.
    ///
    /// Any cards not part of the relevant run target will not be presented to this function in the first
    /// place.
    pub get_weight: Box<dyn Fn(Dynamic, bool) -> Result<f64> + Send + Sync + 'e>,
    /// A closure that, given the user's response to a card, the card's metadata, and whether or not the card has been marked
    /// as difficult, returns new metadata and whether or not the card should now be marked as difficult.
    ///
    /// Note that learn runs do not have the authority to mark cards as starred, or even determine whether or not they are.
    #[allow(clippy::type_complexity)]
    pub adjust_card:
        Box<dyn Fn(String, Dynamic, bool) -> Result<(Dynamic, bool)> + Send + Sync + 'e>,
    /// A closure that produces the default metadata for this method. This is used when a new set is created for
    /// this method to initialise all its cards with metadata that is appropriate to this method. Generally,
    /// methods should keep this as small as possible to minimise the size of sets on-disk.
    pub get_default_metadata: Box<dyn Fn() -> Result<Dynamic> + Send + Sync + 'e>,
}
impl<'e> Method<'e> {
    /// Compiles the given inbuilt script into a full-fledged [`Method`].
    ///
    /// # Errors
    ///
    /// This will fail if the given method name is not the name of an inbuilt method.
    ///
    /// # Panics
    ///
    /// This will panic if compilation fails, as compilation should never fail for an inbuilt method, and this would represent
    /// a bug in California.
    fn from_inbuilt(method_name: &str, engine: &'e Engine) -> Result<Self> {
        if !Method::is_inbuilt(method_name) {
            bail!("provided method name '{method_name}' is not an inbuilt method (are you using the latest version of california?)");
        }
        let script = METHODS
            .get_file(method_name.to_string() + ".rhai")
            .unwrap()
            .contents_utf8()
            .expect("inbuilt method should be utf-8");
        let ast = engine.compile(script).expect(
            "inbuilt method should not panic on compilation (this is a bug in california!)",
        );
        let method = Self::from_ast(method_name, ast, engine)?;

        Ok(method)
    }
    /// Compiles the provided custom Rhai script into a full-fledged [`Method`].
    ///
    /// # Errors
    ///
    /// This will return an error if compiling the provided script fails, or if it does not contain the required elements. See the documentation
    /// of custom methods for details of what these elements are.
    fn from_custom(method_name: &str, method_script: &str, engine: &'e Engine) -> Result<Self> {
        let ast = engine
            .compile(method_script)
            .with_context(|| "compiling custom method script failed")?;
        let method = Self::from_ast(method_name, ast, engine)?;

        Ok(method)
    }
    /// Converts from the AST of a method script to a full method.
    ///
    /// # Errors
    ///
    /// This will explicitly fail if it cannot find the `const RESPONSES` array in the provided AST, but it will create closures that
    /// produce errors when executed if the AST does not contain the required functions `get_weight` and `adjust_card`, or if they
    /// are invalid in some way.
    fn from_ast(method_name: &str, ast: AST, engine: &'e Engine) -> Result<Self> {
        // Extract the closures directly (using the shared engine)
        let ast1 = ast.clone();
        let ast2 = ast.clone();
        let ast3 = ast.clone();
        let get_weight = Box::new(move |method_data, difficult| {
            engine
                .call_fn(
                    &mut Scope::new(),
                    &ast,
                    "get_weight",
                    (method_data, difficult),
                )
                .with_context(|| {
                    "failed to get weight for card (this is a bug in the selected learning method)"
                })
        });
        let adjust_card = Box::new(move |res, method_data, difficult| {
            let res: Array = engine.call_fn(&mut Scope::new(), &ast1, "adjust_card", (res, method_data, difficult)).with_context(|| "failed to adjust card data for last card (this is a bug in the selected learning method)")?;
            let method_data = res.get(0).ok_or(anyhow!("no method data provided from card adjustment (this is a bug in the selected learning method)"))?;
            let difficult = res.get(1).ok_or(anyhow!("no difficulty boolean provided from card adjustment (this is a bug in the selected learning method)"))?.as_bool().map_err(|_| anyhow!("invalid difficulty boolean provided from card adjustment (this is a bug in the selected learning method)"))?;

            Ok((method_data.clone(), difficult))
        });
        let get_default_metadata = Box::new(move || {
            engine.call_fn(&mut Scope::new(), &ast2, "get_default_metadata", ()).with_context(|| "failed to get default metadata for a new card (this is a bug in the selected learning method)")
        });

        // Iterate through all literal constants and find `RESPONSES`
        let mut responses = None;
        for (name, _, value) in ast3.iter_literal_variables(true, false) {
            if name == "RESPONSES" {
                let value = value.into_typed_array().map_err(|_| anyhow!("required constant `RESPONSES` in method script was not an array of strings"))?;
                responses = Some(value);
            }
        }

        if let Some(responses) = responses {
            // Assemble all that into a method
            Ok(Method {
                name: method_name.to_string(),
                responses,
                get_weight,
                adjust_card,
                get_default_metadata,
            })
        } else {
            bail!("method script did not define required constant `RESPONSES`");
        }
    }
    /// Determines if the given method name is inbuilt. This may be unwittingly provided a full method script as well.
    fn is_inbuilt(method: &str) -> bool {
        METHODS.files().any(|file| {
            file.path().file_name().unwrap().to_string_lossy() == method.to_string() + ".rhai"
        })
    }
}

/// A representation of a method that has not yet been created.
#[derive(Clone, Debug)]
pub enum RawMethod {
    /// An inbuilt method, with the name attached.
    Inbuilt(String),
    /// A custom method defined by a Rhai script.
    Custom {
        /// The name of the script. The provided method name **must not** overlap
        /// with that of any other custom method, and it is **strongly** recommended that users prefix their own name or handle in front
        /// of the names of scripts they write to avoid users of these scripts accidentally causing conflicts with scripts written by others.
        ///
        /// E.g. if Alice writes a custom method script and distributes it on the internet with the name `powerlearn-v2`, and Bob starts using
        /// it, but then later decides to use a different script made by Chloe, also called `powerlearn-v2`, California will unwittingly pass
        /// the metadata Alice's script expected to Chloe's, at best causing it to completely fail, and at worst causing all Bob's previous
        /// data to be overwritten irretrievably. This could be avoided if Alice produced `alice/powerlearn-v2` and Chloe produces
        /// `chloe/powerlearn-v2`.
        name: String,
        /// The body of the Rhai script that defines this method, which must contain several key elements (see the documentation of custom
        /// methods to learn more about these).
        body: String,
    },
}
impl RawMethod {
    /// Converts this raw method into a fully-fledged [`Method`].
    ///
    /// # Panics
    ///
    /// This will panic if compiling an inbuilt method fails, as this would be a bug in California. Any other failure will be
    /// gracefully returned as an error.
    pub fn into_method(self, engine: &Engine) -> Result<Method<'_>> {
        match self {
            Self::Inbuilt(name) => Method::from_inbuilt(&name, engine),
            Self::Custom { name, body } => Method::from_custom(&name, &body, engine),
        }
    }
    /// Determines whether or not the given method name or script is inbuilt. This can be used in situations of ambiguity, such
    /// as in a CLI, where a path to a custom script or the name of an inbuilt method may be provided with no immediate distinction.
    pub fn is_inbuilt(method: &str) -> bool {
        Method::is_inbuilt(method)
    }
}
