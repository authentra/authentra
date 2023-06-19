use std::net::{Ipv6Addr, SocketAddr};

use config::{Config, ConfigError, Environment};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AuthentraConfiguration {
    pub listen: ListenConfiguration,
    pub postgres: deadpool_postgres::Config,
    pub secret: String,
    pub allowed_origins: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListenConfiguration {
    pub http: SocketAddr,
    pub metrics: SocketAddr,
}

impl Default for ListenConfiguration {
    fn default() -> Self {
        Self {
            http: SocketAddr::new(std::net::IpAddr::V6(Ipv6Addr::UNSPECIFIED), 8080),
            metrics: SocketAddr::new(std::net::IpAddr::V6(Ipv6Addr::UNSPECIFIED), 3000),
        }
    }
}

impl AuthentraConfiguration {
    pub fn load() -> Result<Self, ConfigError> {
        let default_listen = ListenConfiguration::default();
        let loaded = Config::builder()
            .add_source(Environment::default().separator("_"))
            .add_source(
                Environment::default()
                    .ignore_empty(true)
                    .try_parsing(true)
                    .with_list_parse_key("allowed_origins")
                    .list_separator(" "),
            )
            .set_default("listen.http", default_listen.http.to_string())?
            .set_default("listen.metrics", default_listen.metrics.to_string())?
            .set_default("postgres.port", 5432)?
            .build()?;
        loaded.try_deserialize()
    }
}
