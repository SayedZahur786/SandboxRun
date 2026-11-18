use tower::Layer;
#[tokio::main]
async fn main() -> Result<(), vercel_runtime::Error> {
    let key = std::env::var("SANDBOXRUN_API_KEY")?;
    let engine = sandboxrun::Sandbox::new("runtimes").map_err(|e| e.to_string())?;
    eprintln!("runtime_ready startup_ms={:.3}", engine.startup_ms);
    vercel_runtime::run(
        vercel_runtime::axum::VercelLayer::new().layer(sandboxrun::http::router(engine, key)),
    )
    .await
}
