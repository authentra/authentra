use http::Uri;

use rhai::def_package;
use rhai::export_module;
use rhai::plugin::*;
use rhai::ImmutableString;

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

#[derive(Debug, Clone)]
pub enum RhaiUriError {
    MissingScheme,
    UnsupportedScheme,
}

impl<'a> TryFrom<&'a http::uri::Scheme> for Scheme {
    type Error = RhaiUriError;

    fn try_from(value: &'a http::uri::Scheme) -> Result<Self, Self::Error> {
        Ok(if value == &http::uri::Scheme::HTTP {
            println!("HTTP");
            Self::Http
        } else if value == &http::uri::Scheme::HTTPS {
            println!("HTTPS");
            Self::Https
        } else {
            return Err(RhaiUriError::UnsupportedScheme);
        })
    }
}

impl RhaiUri {
    pub fn create(scheme: Scheme, host: String, uri: &Uri) -> Result<RhaiUri, RhaiUriError> {
        let port = uri.port_u16();
        let path = uri.path();
        let query = uri.query();
        Ok(RhaiUri {
            uri: uri.to_string().into(),
            scheme,
            host: host.into(),
            port,
            path: path.into(),
            query: query.map(Into::into),
        })
    }
}

def_package! {
    pub UriPackage(module) {
        combine_with_exported_module!(module, "Uri", uri_module);
        combine_with_exported_module!(module, "Scheme", scheme_module);
    }
}

#[export_module]
mod uri_module {

    pub type Uri = super::RhaiUri;
    pub type Scheme = super::Scheme;

    #[rhai_fn(global, pure)]
    pub fn to_string(obj: &mut Uri) -> ImmutableString {
        obj.uri.clone()
    }

    #[rhai_fn(global, get = "scheme", pure)]
    pub fn get_scheme(obj: &mut Uri) -> Scheme {
        obj.scheme.clone()
    }

    #[rhai_fn(global, get = "host", pure)]
    pub fn get_host(obj: &mut Uri) -> ImmutableString {
        obj.host.clone()
    }
    #[rhai_fn(global, get = "port", pure)]
    pub fn get_port(obj: &mut Uri) -> u16 {
        match obj.port {
            Some(p) => p,
            None => match obj.scheme {
                Scheme::Https => 443,
                Scheme::Http => 80,
            },
        }
    }

    #[rhai_fn(global, get = "path", pure)]
    pub fn get_path(obj: &mut Uri) -> ImmutableString {
        obj.path.clone()
    }
    #[rhai_fn(global, get = "query", pure)]
    pub fn get_query(obj: &mut Uri) -> Option<ImmutableString> {
        obj.query.clone()
    }
}

#[export_module]
mod scheme_module {
    #[rhai_fn(global, pure)]
    pub fn is_http(scheme: &mut Scheme) -> bool {
        *scheme == Scheme::Http
    }
    #[rhai_fn(global, pure)]
    pub fn is_https(scheme: &mut Scheme) -> bool {
        *scheme == Scheme::Https
    }
}

#[cfg(test)]
mod test {

    use http::Uri;
    use rhai::ImmutableString;

    use super::RhaiUri;
    use super::Scheme;
    use crate::{tests::preload::*, uri::UriPackage};

    fn uri(scheme: Scheme, uri: &'static str) -> RhaiUri {
        RhaiUri::create(scheme, "host".into(), &Uri::from_static(uri))
            .expect("Failed to construct uri")
    }

    eval_test!(test_to_string("uri": uri(Scheme::Http,"http://host/this/is/a/path")) -> String | ("http://host/this/is/a/path".to_owned()): "uri.to_string()", UriPackage);

    eval_test!(test_http_port("uri": uri(Scheme::Http,"http://host/")) -> u16 | (80): "uri.port", UriPackage);
    eval_test!(test_https_port("uri": uri(Scheme::Https,"https://host/")) -> u16 | (443): "uri.port", UriPackage);
    eval_test!(test_custom_port("uri": uri(Scheme::Http,"http://host:90/")) -> u16 | (90): "uri.port", UriPackage);

    eval_test!(test_host("uri": uri(Scheme::Http,"http://host/")) -> String | ("host".to_owned()): "uri.host", UriPackage);
    eval_test!(test_path("uri": uri(Scheme::Http,"http://host/this/is/a/path")) -> String | ("/this/is/a/path".to_owned()): "uri.path", UriPackage);
    eval_test!(test_query("uri": uri(Scheme::Http,"http://host/path?this=is&a=query")) -> Option<ImmutableString> | (Some(ImmutableString::from("this=is&a=query"))): "uri.query", UriPackage);

    eval_test!(test_scheme("uri": uri(Scheme::Http,"http://host/")) -> Scheme | (Scheme::Http): "uri.scheme", UriPackage);
    eval_test!(test_scheme_is_http("uri": uri(Scheme::Http,"http://host/")) -> bool | (true): "uri.scheme.is_http()", UriPackage);
    eval_test!(test_scheme_is_https("uri": uri(Scheme::Https,"https://host/")) -> bool | (true): "uri.scheme.is_https()", UriPackage);
}
