use http::Uri;
use once_cell::sync::Lazy;
use rhai::export_module;
use rhai::plugin::*;
use rhai::ImmutableString;
use std::sync::Arc;

use crate::TryAsRef;

#[derive(Debug, Clone, PartialEq)]
pub struct RhaiUri {
    uri: ImmutableString,
    scheme: Scheme,
    host: ImmutableString,
    port: Option<u16>,
    path: ImmutableString,
    query: Option<ImmutableString>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Scheme {
    Http,
    Https,
}

pub enum RhaiUriError {
    MissingScheme,
    UnsupportedScheme,
    MissingHost,
}

impl<'a> TryFrom<&'a http::uri::Scheme> for Scheme {
    type Error = RhaiUriError;

    fn try_from(value: &'a http::uri::Scheme) -> Result<Self, Self::Error> {
        Ok(if value == &http::uri::Scheme::HTTP {
            Self::Http
        } else if value == &http::uri::Scheme::HTTPS {
            Self::Https
        } else {
            return Err(RhaiUriError::UnsupportedScheme);
        })
    }
}

impl TryAsRef<RhaiUri> for Uri {
    type Error = RhaiUriError;

    fn try_as_ref(&self) -> Result<RhaiUri, Self::Error> {
        let scheme = self.scheme().ok_or(RhaiUriError::MissingScheme)?;
        let scheme = Scheme::try_from(scheme)?;
        let host = self.host().ok_or(RhaiUriError::MissingHost)?;
        let uri = self.to_string();
        let port = self.port_u16();
        let path = self.path();
        let query = self.query();
        Ok(RhaiUri {
            uri: uri.into(),
            scheme,
            host: host.into(),
            port,
            path: path.into(),
            query: query.map(Into::into),
        })
    }
}

pub static MODULE: Lazy<Arc<Module>> = Lazy::new(|| Arc::new(exported_module!(uri_module)));

#[export_module]
mod uri_module {

    pub type Uri = super::RhaiUri;
    pub type Scheme = super::Scheme;

    #[rhai_fn(get = "uri", pure)]
    pub fn get_uri(obj: &mut Uri) -> ImmutableString {
        obj.uri.clone()
    }

    #[rhai_fn(get = "scheme", pure)]
    pub fn get_scheme(obj: &mut Uri) -> Scheme {
        obj.scheme.clone()
    }

    #[rhai_fn(get = "host", pure)]
    pub fn get_host(obj: &mut Uri) -> ImmutableString {
        obj.host.clone()
    }
    #[rhai_fn(get = "port", pure)]
    pub fn get_port(obj: &mut Uri) -> Option<u16> {
        obj.port
    }

    #[rhai_fn(get = "path", pure)]
    pub fn get_path(obj: &mut Uri) -> ImmutableString {
        obj.path.clone()
    }
    #[rhai_fn(get = "query", pure)]
    pub fn get_query(obj: &mut Uri) -> Option<ImmutableString> {
        obj.query.clone()
    }
}

mod test {}
