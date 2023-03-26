use authust_server::test_util::*;
use http::{Method, StatusCode};
use tower::ServiceExt;

#[tokio::test]
pub async fn hello_world() {
    run_test(|app| async {
        let request = request(Method::GET, "/test");
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(body_to_string(response).await, "Hello, World!");
    })
    .await;
    tracing::info!("End");
}
