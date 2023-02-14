use std::net::{Ipv6Addr, SocketAddr};

use config::{Config, ConfigError};
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct AuthustConfiguration {
    pub allowed_hosts: &'static [&'static str],
}

#[derive(Debug, Clone, Deserialize)]
pub struct InternalAuthustConfiguration {
    pub listen: ListenConfiguration,
    pub postgres: PostgresConfiguration,
    pub secret: String,
    pub jaeger_endpoint: Option<String>,
    // pub allowed_hosts: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListenConfiguration {
    pub http: SocketAddr,
    pub https: Option<SocketAddr>,
    pub metrics: SocketAddr,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PostgresConfiguration {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    pub password: String,
}

impl Default for ListenConfiguration {
    fn default() -> Self {
        Self {
            http: SocketAddr::new(std::net::IpAddr::V6(Ipv6Addr::UNSPECIFIED), 8080),
            https: None,
            metrics: SocketAddr::new(std::net::IpAddr::V6(Ipv6Addr::UNSPECIFIED), 3000),
        }
    }
}

impl From<InternalAuthustConfiguration> for AuthustConfiguration {
    fn from(_value: InternalAuthustConfiguration) -> Self {
        Self {
            allowed_hosts: leak_vec(vec![]), // allowed_hosts: leak_vec(value.allowed_hosts),
        }
    }
}

fn leak_string(str: String) -> &'static str {
    Box::leak(str.into_boxed_str())
}

fn leak_vec(vec: Vec<String>) -> &'static [&'static str] {
    Box::leak(
        vec.into_iter()
            .map(|v| leak_string(v))
            .collect::<Vec<&'static str>>()
            .into_boxed_slice(),
    )
}

impl InternalAuthustConfiguration {
    pub fn load() -> Result<Self, ConfigError> {
        let default_listen = ListenConfiguration::default();
        let loaded = Config::builder()
            .add_source(
                config::Environment::with_prefix("AUTHUST")
                    .ignore_empty(true)
                    .separator("__")
                    .prefix_separator("_"),
            )
            .set_default("listen.http", default_listen.http.to_string())?
            .set_default("listen.metrics", default_listen.metrics.to_string())?
            .set_default("postgres.port", 5432)?
            .build()?;
        loaded.try_deserialize()
    }
}
