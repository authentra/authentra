use base64::{prelude::BASE64_URL_SAFE_NO_PAD, Engine as _};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rhai::{Engine, ParseError, Position, Scope, AST};
use std::sync::Arc;

pub mod context;
pub mod uri;

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
    engine.register_global_module(uri::MODULE.clone());
    engine
}

static DEFAULT_ENGINE: Lazy<Engine> = Lazy::new(|| create_engine());

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
    let ast = DEFAULT_ENGINE.compile_expression(expression)?;
    Ok(ast)
}

pub fn execute(ast: AST) {
    let mut engine = create_engine();
    let mut out: Arc<Mutex<Vec<LogEntry>>> = Arc::new(Mutex::new(Vec::new()));
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
    let scope = Scope::new();
}

#[derive(Debug, Clone)]
pub struct ExecutionOptions {
    pub debug: bool,
}

pub enum LogEntry {
    Text(String),
    Debug(RhaiDebugEntry),
}

#[derive(Debug, Clone)]
pub struct RhaiDebugEntry {
    pub text: String,
    pub source: Option<String>,
    pub position: Position,
}

pub struct ExecutionOutput {
    debugs: Vec<RhaiDebugEntry>,
    prints: Vec<String>,
}
