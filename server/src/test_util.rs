use std::fmt::Debug;

use axum::body::{Body, HttpBody};
use axum::Router;
use config::ConfigError;
use deadpool_postgres::{Config, Pool};
use futures::Future;
use http::{header::HOST, Method, Request};
use once_cell::sync::Lazy;
use tokio::sync::Mutex;

use crate::{
    config::InternalAuthustConfiguration,
    setup::{create_database_pool, create_state, router, run_migrations},
};

static CONFIGURATION: Lazy<Result<InternalAuthustConfiguration, ConfigError>> =
    Lazy::new(InternalAuthustConfiguration::load);

static RAN_MIGRATIONS: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

pub async fn run_test<F, T>(func: F) -> T::Output
where
    T: Future + Send + 'static,
    T::Output: Send + 'static,
    F: FnOnce(Router) -> T,
{
    tracing_subscriber::fmt().init();
    let configuration = match &*CONFIGURATION {
        Ok(config) => config.clone(),
        Err(err) => {
            panic!("Failed to load configuration! {err}");
        }
    };
    let pool = configure_database(configuration.postgres.clone()).await;
    let state = create_state(configuration, pool).await;
    let router = router(state).await;
    let future = func(router);
    future.await
}

pub fn request(method: Method, path: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .header(HOST, "127.0.0.1")
        .body(Body::empty())
        .unwrap()
}

async fn configure_database(mut config: Config) -> Pool {
    let mut pool = config.pool.unwrap_or_default();
    pool.max_size = 1;
    config.pool = Some(pool);
    let pool = create_database_pool(config);
    let mut lock = RAN_MIGRATIONS.lock().await;
    let mut client = pool.get().await.expect("Failed to get connection");
    if !*lock {
        run_migrations(&mut client).await;
        *lock = true;
    }
    let statement = client
        .prepare_cached("begin")
        .await
        .expect("Failed to cache 'begin' query");
    client
        .execute(&statement, &[])
        .await
        .expect("Failed to begin transaction");
    pool
}
pub async fn body_to_string<B>(body: B) -> String
where
    B: HttpBody,
    B::Error: Debug,
{
    let body = hyper::body::to_bytes(body).await.unwrap();
    String::from_utf8_lossy(&body).to_string()
}
