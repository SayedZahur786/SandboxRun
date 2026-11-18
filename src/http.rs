use crate::{ExecutionRequest, Limits, MAX_BODY, Sandbox};
use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde_json::json;
use std::sync::Arc;
use subtle::ConstantTimeEq;
#[derive(Clone)]
struct App {
    engine: Arc<Sandbox>,
    key: Arc<String>,
    permits: Arc<tokio::sync::Semaphore>,
}
fn error(status: StatusCode, msg: &str) -> Response {
    (status, Json(json!({"error":msg}))).into_response()
}
async fn execute(State(app): State<App>, headers: HeaderMap, body: Bytes) -> Response {
    let provided = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");
    if !bool::from(provided.as_bytes().ct_eq(app.key.as_bytes())) {
        return error(
            StatusCode::UNAUTHORIZED,
            "A valid bearer API key is required",
        );
    }
    let permit = match app.permits.clone().try_acquire_owned() {
        Ok(p) => p,
        Err(_) => {
            return error(
                StatusCode::TOO_MANY_REQUESTS,
                "Execution capacity reached; retry later",
            );
        }
    };
    let req: ExecutionRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(_) => return error(StatusCode::BAD_REQUEST, "Invalid execution request"),
    };
    if let Err(e) = req.validate() {
        return error(StatusCode::BAD_REQUEST, &e.to_string());
    }
    let language = match req.language {
        crate::Language::Python => "python",
        crate::Language::Javascript => "javascript",
    };
    let result = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        app.engine.execute(req)
    })
    .await;
    match result {
        Ok(Ok(receipt)) => {
            println!(
                "{}",
                json!({"event":"execution","language":language,"status":receipt.status,"duration_ms":receipt.duration_ms})
            );
            Json(receipt).into_response()
        }
        _ => error(StatusCode::SERVICE_UNAVAILABLE, "Runtime unavailable"),
    }
}
pub fn capabilities() -> serde_json::Value {
    json!({"languages":[{"id":"python","runtime":"CPython 3.15.2 + wasi-vfs"},{"id":"javascript","runtime":"QuickJS-NG 0.18.0"}],"defaults":Limits::default(),"maximums":{"fuel":5_000_000_000u64,"memory_bytes":268435456,"wall_time_ms":5000,"output_bytes":65536},"filesystem":false,"network":false,"persistence":false,"max_code_bytes":65536,"max_stdin_bytes":65536,"output_limit_scope":"per_stream","memory_limit_scope":"guest_linear_memory","cpu_limit_scope":"wasmtime_fuel","concurrency_per_instance":2})
}
async fn events(body: Bytes) -> Response {
    let value = serde_json::from_slice::<serde_json::Value>(&body).unwrap_or_default();
    match value.get("event").and_then(|x| x.as_str()) {
        Some(event @ ("page_view" | "run_clicked"))
            if value.as_object().is_some_and(|o| o.len() == 1) =>
        {
            println!("{}", json!({"event":event}));
            StatusCode::NO_CONTENT.into_response()
        }
        _ => error(StatusCode::BAD_REQUEST, "Unsupported event"),
    }
}
pub fn router(engine: Sandbox, key: String) -> Router {
    assert!(
        key.len() >= 32,
        "SANDBOXRUN_API_KEY must have at least 32 characters"
    );
    let app = App {
        engine: Arc::new(engine),
        key: Arc::new(key),
        permits: Arc::new(tokio::sync::Semaphore::new(2)),
    };
    Router::new()
        .route("/api/run", post(execute))
        .route("/v1/execute", post(execute))
        .route("/api/capabilities", get(|| async { Json(capabilities()) }))
        .route("/v1/capabilities", get(|| async { Json(capabilities()) }))
        .route(
            "/api/health",
            get(|| async { Json(json!({"status":"ready"})) }),
        )
        .route("/api/events", post(events))
        .layer(DefaultBodyLimit::max(MAX_BODY))
        .with_state(app)
        .fallback_service(tower_http::services::ServeDir::new("web"))
        .layer(tower_http::set_header::SetResponseHeaderLayer::overriding(
            axum::http::header::CACHE_CONTROL,
            axum::http::HeaderValue::from_static("no-store"),
        ))
        .layer(tower_http::set_header::SetResponseHeaderLayer::overriding(
            axum::http::header::X_CONTENT_TYPE_OPTIONS,
            axum::http::HeaderValue::from_static("nosniff"),
        ))
}
