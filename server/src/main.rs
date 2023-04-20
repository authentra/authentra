use std::{
    net::{SocketAddr, TcpListener},
    ops::DerefMut,
};

use axum::{Router, Server};
use deadpool_postgres::{Config, Object, Pool};
use tokio::signal;
use tracing::info;

use crate::{config::AuthustStartupConfiguration, middleware::auth::AuthState};

mod api;
mod config;
pub mod middleware;
mod telemetry;
pub mod utils;

#[tokio::main]
async fn main() {
    main_tokio().await;
}

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("./migrations/");
}

async fn main_tokio() {
    let configuration = AuthustStartupConfiguration::load().unwrap();
    telemetry::setup_tracing();

    let pool = create_database_pool(configuration.postgres.clone());
    {
        let mut conn = pool.get().await.expect("Failed to get database connection");
        run_migrations(&mut conn).await;
    }

    let auth_state = AuthState::new(configuration.secret);

    let router = Router::new().nest("/api", api::setup_router(auth_state));
    let addr = SocketAddr::new("::".parse().unwrap(), 8000);
    let shutdown_future = async { signal::ctrl_c().await.unwrap() };
    // let socket =
    //     socket2::Socket::new(Domain::for_address(addr), Type::STREAM, Some(Protocol::TCP)).unwrap();
    // socket.set_only_v6(false).unwrap();
    // socket.bind(&SockAddr::from(addr)).unwrap();
    // socket.listen(128).unwrap();
    // let tcp: TcpListener = socket.into();
    // let tcp = TcpListener::bind(addr).unwrap();
    // Server::from_tcp(tcp)
    // .unwrap()
    Server::bind(&addr)
        .serve(router.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown_future)
        .await
        .expect("Server crashed");
    info!("Server shutdown");
    opentelemetry::global::shutdown_tracer_provider();
}

pub fn create_database_pool(configuration: Config) -> Pool {
    configuration
        .create_pool(
            Some(deadpool_postgres::Runtime::Tokio1),
            tokio_postgres::NoTls,
        )
        .expect("Failed to create pool")
}

pub async fn run_migrations(client: &mut Object) {
    info!("Running migrations on database...");
    let report = embedded::migrations::runner()
        .run_async(client.as_mut().deref_mut())
        .await
        .expect("Failed to run migrations");
    info!("Database migration report: {report:?}");
}
