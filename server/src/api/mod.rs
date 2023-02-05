use http::header::HeaderName;

pub use v1::setup_api_v1;

pub mod csrf;
pub mod sql_tx;
mod v1;
pub use v1::ApiError as V1ApiError;
pub use v1::ApiErrorKind as V1ApiErrorKind;
pub const CSRF_HEADER: HeaderName = HeaderName::from_static("x-csrf-token");
