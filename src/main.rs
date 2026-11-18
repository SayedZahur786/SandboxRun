#[tokio::main]
async fn main()->anyhow::Result<()> {
 let key=std::env::var("SANDBOXRUN_API_KEY").expect("Set SANDBOXRUN_API_KEY (32+ chars)");
 let engine=sandboxrun::Sandbox::new("runtimes").map_err(|e|anyhow::anyhow!("{e:#}"))?;
 eprintln!("runtime_ready startup_ms={:.3}",engine.startup_ms);
 let listener=tokio::net::TcpListener::bind("127.0.0.1:8787").await?;
 axum::serve(listener,sandboxrun::http::router(engine,key)).await?;Ok(())
}
