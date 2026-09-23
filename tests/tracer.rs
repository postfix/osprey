//! Slice 1's witness: the whole wire — CLI configuration, router, error mapping and
//! a real policy call — against a server that is actually listening.

mod common;

use common::TestServer;
use serde_json::Value;

#[tokio::test]
async fn health_live_answers_200() {
    let server = TestServer::start().await;

    let response = server.get("/health/live").await;

    assert_eq!(response.status().as_u16(), 200);
    server.shutdown().await;
}

#[tokio::test]
async fn health_ready_answers_503_while_no_blocklist_is_loaded() {
    let server = TestServer::start().await;

    let response = server.get("/health/ready").await;

    assert_eq!(response.status().as_u16(), 503);
    server.shutdown().await;
}

#[tokio::test]
async fn npm_package_answers_503_policy_unavailable() {
    let server = TestServer::start().await;

    let response = server.get("/npm/left-pad").await;

    assert_eq!(response.status().as_u16(), 503);
    assert_eq!(
        response
            .headers()
            .get("cache-control")
            .and_then(|value| value.to_str().ok()),
        Some("no-store")
    );

    let body = response.text().await.expect("a response body");
    let body: Value = serde_json::from_str(&body).expect("a JSON error body");
    assert_eq!(body["error"], Value::from("POLICY_UNAVAILABLE"));
    assert!(
        body["reason"]
            .as_str()
            .is_some_and(|reason| !reason.is_empty()),
        "the error body carries a non-empty reason: {body}"
    );
    assert!(
        body["request_id"].as_str().is_some_and(|id| !id.is_empty()),
        "the error body carries a non-empty request_id: {body}"
    );

    server.shutdown().await;
}

#[tokio::test]
async fn an_unknown_path_answers_404() {
    let server = TestServer::start().await;

    let response = server.get("/not-a-route").await;

    assert_eq!(response.status().as_u16(), 404);
    server.shutdown().await;
}
