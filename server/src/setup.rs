use std::{ops::DerefMut, sync::Arc, time::Duration};

use axum::{error_handling::HandleErrorLayer, routing::get, Router};
use deadpool_postgres::{Config, Object, Pool};
use jsonwebtoken::{DecodingKey, EncodingKey};
use storage::Storage;
use tower::ServiceBuilder;
use tower_cookies::CookieManagerLayer;
use tracing::info;

use crate::{
    api::{setup_api_v1, AuthServiceData},
    config::InternalAuthustConfiguration,
    embedded,
    executor::FlowExecutor,
    handle_timeout_error, hello_world,
    interface::setup_interface_router,
    otel_middleware::{otel_layer, ExtensionLayer},
    service::{policy::PolicyService, user::UserService},
    Defaults, InternalSharedState, SharedState,
};

pub async fn create_state(config: InternalAuthustConfiguration, pool: Pool) -> SharedState {
    let storage = Storage::new(pool.clone()).await.unwrap();
    let policies = PolicyService::new(storage.clone());
    let defaults = Defaults::new(&storage, pool.clone()).await;
    let executor = FlowExecutor::new(Arc::new(storage.clone()), policies.clone());
    let users = UserService::new();
    let internal_state = InternalSharedState {
        users,
        executor,
        auth_data: AuthServiceData {
            encoding_key: EncodingKey::from_secret(config.secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(config.secret.as_bytes()),
        },
        storage,
        defaults: Arc::new(defaults),
        policies,
    };

    SharedState(Arc::new(internal_state))
}

pub async fn router(state: SharedState) -> Router<()> {
    #[cfg(debug_assertions)]
    #[cfg(feature = "dev-mode")]
    let cors = tower_http::cors::CorsLayer::very_permissive();
    let service = ServiceBuilder::new()
        .layer(ExtensionLayer)
        .layer(otel_layer())
        .layer(CookieManagerLayer::new())
        .layer(HandleErrorLayer::new(handle_timeout_error))
        .timeout(Duration::from_secs(1));
    #[cfg(debug_assertions)]
    #[cfg(feature = "dev-mode")]
    let service = service.layer(cors);

    Router::new()
        .route("/test", get(hello_world))
        .nest("/api/v1", setup_api_v1(state.clone()).await)
        .nest("/", setup_interface_router())
        .layer(service)
        .with_state(state)
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
