use authust_server::{
    config::{InternalAuthustConfiguration, ListenConfiguration},
    setup::{create_database_pool, create_state, router, run_migrations},
    tracing_setup::setup_tracing,
    INTERFACE_BASE_URI,
};

use axum::Router;
use futures::{Future, FutureExt};

use tokio::signal;
use tracing::info;

#[tokio::main]
async fn main() {
    setup().await;
}

async fn setup() {
    setup_tracing();

    let configuration = InternalAuthustConfiguration::load().expect("Failed to load configuration");
    info!("Setting up database...");

    // Initialize INTERFACE_BASE_URI
    let pool = create_database_pool(configuration.postgres.clone());
    {
        let mut conn = pool.get().await.expect("Failed to get database connection");
        run_migrations(&mut conn).await;
    }

    let listen = configuration.listen.clone();
    let state = create_state(configuration, pool).await;
    let router = router(state).await;

    let shutdown_future = signal::ctrl_c().map(|fut| {
        fut.expect("Error occurred while waiting for Ctrl+C signal");
        tracing::info!("Received shutdown signal");
    });
    #[allow(unused_must_use)]
    {
        INTERFACE_BASE_URI.len();
    }

    let app_future = start_server(listen, router, shutdown_future);
    app_future.await;
    tracing::info!("Shutting down opentelemetry provider");
    opentelemetry::global::shutdown_tracer_provider();
    tracing::info!("Shutdown complete");
}

async fn start_server(
    listen: ListenConfiguration,
    router: Router<()>,
    future: impl Future<Output = ()>,
) {
    let bind = axum::Server::bind(&listen.http);
    info!("Listening on {}...", listen.http);
    bind.serve(router.into_make_service())
        .with_graceful_shutdown(future)
        .await
        .expect("Server crashed");
    // server.await.expect("Server crashed");
}
