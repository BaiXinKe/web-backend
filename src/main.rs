//! src/main.rs

use std::error::Error;

use blog_backend::{
    configuration,
    telemetry::{get_subscriber, init_subscriber},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let subscriber = get_subscriber("zero2prod".to_string(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = configuration::get_configuration().expect("Failed to read configuration");
    let application = blog_backend::startup::Application::build(configuration).await?;

    application.run_until_stopped().await?;

    Ok(())
}
