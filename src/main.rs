//! src/main.rs

use std::{error::Error, net::TcpListener};

use blog_backend::{
    configuration,
    telemetry::{get_subscriber, init_subscriber},
};
use sqlx::PgPool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let subscriber = get_subscriber("zero2prod".to_string(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = configuration::get_configuration().expect("Failed to read configuration");
    let connection_pool = PgPool::connect_with(configuration.database.with_db())
        .await
        .expect("Failed to connect to Postgres.");

    let address = format!("127.0.0.1:{}", configuration.application.port);
    let listener = TcpListener::bind(address)?;
    blog_backend::startup::run(listener, connection_pool)?.await?;

    Ok(())
}
