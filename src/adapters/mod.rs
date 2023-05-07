use std::path::PathBuf;
use include_dir::{Dir, include_dir};
use rhai::{AST, Engine};

/// The `src/adapters` directory that includes this file.
static ADAPTERS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/adapters");

/// Parses the given adapter the user provided on the command line, resolving it to AST compiled with the globally stored
/// Rhai engine. If the user provides the name of an inbuilt adapter, that will be used, otherwise a script the user
/// provides will be used.
///
/// Note that, unlike for methods, users will typically provide their own custom adapters, and there are far fewer inbuilt
/// adapters.
///
/// # Errors
///
/// This will return an error if there is any problem in compilation, or if the user provides an invalid path for a custom
/// adapter script.
pub fn parse_adapter(adapter: &str, engine: &Engine) -> Result<AST, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let ast = if ADAPTERS
        .files()
        .any(|file| {
            file.path().file_name().unwrap().to_string_lossy() == adapter.to_string() + ".rhai"
        })
    {
        // Inbuilt adapter
        let script = ADAPTERS
            .get_file(adapter.to_string() + ".rhai")
            .unwrap()
            .contents_utf8()
            .expect("inbuilt adapter should be utf-8");
        engine.compile(script).expect("inbuilt adapter should not panic on compilation (this is a bug in california!)")
    } else {
        // Custom file, check if it's valid and then compile it
        let adapter = PathBuf::from(adapter);
        if !adapter.exists() || !adapter.is_file() {
            return Err("provided adapter is not inbuilt, and does not represent a valid path to a custom adapter script (maybe you're using an adapter in a newer version of california?)".into())
        }
        engine.compile_file(adapter).map_err(|err| format!("compiling custom adapter script failed: {err}"))?
    };

    Ok(ast)
}
