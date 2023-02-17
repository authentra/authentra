use ::rhai::{
    packages::{
        ArithmeticPackage, BasicFnPackage, BasicStringPackage, LanguageCorePackage, Package,
    },
    Engine, EvalAltResult, ParseError, Position, Scope, AST,
};
use base64::{prelude::BASE64_URL_SAFE_NO_PAD, Engine as _};
use network::NetworkPackage;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::sync::Arc;
use uri::RhaiUri;

pub mod context;
pub mod network;
pub mod request;
pub mod uri;
pub mod user;
pub mod rhai {
    pub use ::rhai::*;
}

pub trait TryAsRef<T: Sized> {
    type Error;

    fn try_as_ref(&self) -> Result<T, Self::Error>;
}

pub fn create_engine() -> Engine {
    let mut engine = Engine::new_raw();
    engine
        .set_allow_loop_expressions(false)
        .set_allow_looping(false)
        .set_strict_variables(true);
    engine.set_max_string_size(128).set_max_operations(1000);
    register_packages(&mut engine);
    engine
}

static DEFAULT_ENGINE: Lazy<Engine> = Lazy::new(|| create_engine());

#[derive(Debug)]
pub enum ExpressionCompilationError {
    InvalidBase64,
    Parse(ParseError),
}

impl From<base64::DecodeError> for ExpressionCompilationError {
    fn from(_: base64::DecodeError) -> Self {
        Self::InvalidBase64
    }
}

impl From<ParseError> for ExpressionCompilationError {
    fn from(value: ParseError) -> Self {
        Self::Parse(value)
    }
}

pub fn compile(expression: &str) -> Result<AST, ExpressionCompilationError> {
    let bytes = BASE64_URL_SAFE_NO_PAD.decode(expression)?;
    let expression = String::from_utf8_lossy(&bytes);
    let ast = DEFAULT_ENGINE.compile(expression)?;
    Ok(ast)
}

fn register_packages(engine: &mut Engine) {
    LanguageCorePackage::new().register_into_engine(engine);
    ArithmeticPackage::new().register_into_engine(engine);
    BasicStringPackage::new().register_into_engine(engine);
    BasicFnPackage::new().register_into_engine(engine);
    NetworkPackage::new().register_into_engine(engine);
}

pub fn execute<'a, F: FnOnce() -> Scope<'a>>(ast: &AST, create_scope: F) -> ExecutionResult {
    let mut engine = create_engine();
    let out: Arc<Mutex<Vec<LogEntry>>> = Arc::new(Mutex::new(Vec::new()));
    {
        let out = out.clone();
        engine.on_print(move |txt| out.lock().push(LogEntry::Text(txt.into())));
    }
    {
        let out = out.clone();
        engine.on_debug(move |text, source, pos| {
            out.lock().push(LogEntry::Debug(RhaiDebugEntry {
                text: text.into(),
                source: source.map(Into::into),
                position: pos,
            }));
        });
    }

    let engine = engine;

    let mut scope = create_scope();
    let result = engine.eval_ast_with_scope::<bool>(&mut scope, ast);
    let output: Vec<LogEntry> = out.lock().iter().cloned().collect();
    ExecutionResult { output, result }
}

#[derive(Debug)]
pub struct ExecutionResult {
    pub output: Vec<LogEntry>,
    pub result: Result<bool, Box<EvalAltResult>>,
}

#[derive(Debug, Clone)]
pub struct ExecutionOptions {
    pub debug: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LogEntry {
    Text(String),
    Debug(RhaiDebugEntry),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RhaiDebugEntry {
    pub text: String,
    pub source: Option<String>,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExecutionContext {
    pub uri: RhaiUri,
}

#[cfg(test)]
macro_rules! register_package {
    ($engine:expr, $package:ty) => {
        use rhai::packages::Package;
        <$package>::new().register_into_engine($engine);
    };
}

#[cfg(test)]
macro_rules! eval_test {
    ($fn_name:ident, $rt:ty, ($rt_expr:expr): $expr:literal, $($package:ty),+ ) => {
        #[test]
        fn $fn_name() {
            let mut engine = Engine::new_raw();
            register_package!(&mut engine, $($package),+);
            let mut scope = concat_idents::concat_idents!(fn_name = $fn_name, _scope {
                fn_name()
            });
            let res = engine.eval_with_scope::<$rt>(&mut scope, $expr).expect("Rhai execution failed");
            assert_eq!($rt_expr, res);
        }
    };
    ($fn_name:ident -> $rt:ty | ($rt_expr:expr): $expr:literal, $($package:ty),+ ) => {
        #[test]
        fn $fn_name() {
            let mut engine = Engine::new_raw();
            register_package!(&mut engine, $($package),+);
            let mut scope = Scope::new();
            let res = engine.eval_with_scope::<$rt>(&mut scope, $expr).expect("Rhai execution failed");
        }
    };
    ($fn_name:ident($value_name:literal: $value:expr) -> $rt:ty | ($rt_expr:expr): $expr:literal, $($package:ty),+ ) => {
        #[test]
        fn $fn_name() {
            let mut engine = Engine::new_raw();
            register_package!(&mut engine, $($package),+);
            let mut scope = Scope::new();
            scope.push_constant($value_name, $value);
            let res = engine.eval_with_scope::<$rt>(&mut scope, $expr).expect("Rhai execution failed");
            assert_eq!($rt_expr, res);
        }
    }
}

#[cfg(test)]
pub(crate) use eval_test;
#[cfg(test)]
pub(crate) use register_package;

#[cfg(test)]
pub(crate) fn simple_eval(expr: &str) -> crate::ExecutionResult {
    let ast = compile(BASE64_URL_SAFE_NO_PAD.encode(expr).as_str()).expect("Compilation failed");
    execute(&ast, Scope::new)
}

#[cfg(test)]
mod tests {
    use crate::{simple_eval, LogEntry};

    pub mod preload {
        pub(crate) use crate::{eval_test, register_package};
        pub(crate) use rhai::{Engine, Scope};
    }

    #[test]
    fn hello_world() {
        let res = simple_eval(
            r#"
                print(1);
                true
            "#,
        );
        assert_eq!(Some(true), res.result.ok());
        assert_eq!(vec![LogEntry::Text("1".into())], res.output);
    }
}
