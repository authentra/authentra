use std::{
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
};

use reqwest::{Client, Request};
use tokio::time::Instant;

use crate::cli::LoadTestArgs;

pub async fn load_test(args: LoadTestArgs) {
    let client = Box::leak(Box::new(
        reqwest::ClientBuilder::new()
            .user_agent("authust-tools")
            .build()
            .unwrap(),
    ));

    let request = client
        .get("http://127.0.0.1:8080/api/v1/flow/executor/test-flow")
        .build()
        .unwrap();
    let current_requests = Box::leak(Box::new(AtomicU32::new(0)));
    for i in 0..args.requests {
        while current_requests.load(Ordering::Relaxed) > args.max_concurrent {
            tokio::time::sleep(Duration::from_nanos(200)).await;
        }
        current_requests.fetch_add(1, Ordering::Relaxed);
        tokio::time::sleep(Duration::from_nanos(100)).await;
        tokio::spawn(make_request(
            client,
            request.try_clone().unwrap(),
            current_requests,
            i,
        ));
    }
}

async fn make_request(client: &Client, request: Request, concurrent: &AtomicU32, n: u32) {
    let time = Instant::now();
    match client.execute(request).await {
        Ok(res) => {
            let time = Instant::now() - time;
            println!("Completed request {n} Took: {}ms", time.as_millis());
        }
        Err(err) => {
            eprintln!("Request {n} failed! {err}");
        }
    }
    concurrent.fetch_sub(1, Ordering::Relaxed);
}
