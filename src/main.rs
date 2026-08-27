use tracing_subscriber::EnvFilter;

mod app;

#[tokio::main]
async fn main() {
  // enable dotenv
  dotenvy::dotenv().ok();

  // initialize tracing
  tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::from_default_env())
    .init();

  if let Err(error) = app::run().await {
    tracing::error!(%error, "application failed");
    std::process::exit(1);
  }
}
