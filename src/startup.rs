//! src/startup.rs

use std::net::TcpListener;

use axum::{
    routing::{get, post, IntoMakeService},
    Router,
};
use hyper::server::conn::AddrIncoming;
use sqlx::PgPool;

use crate::routes::{health_check, subscribe};

pub fn run(
    listener: TcpListener,
    db_pool: PgPool,
) -> std::result::Result<axum::Server<AddrIncoming, IntoMakeService<Router>>, std::io::Error> {
    // let db_pool = Arc::new(db_pool);

    let router = axum::Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .with_state(db_pool.clone());

    let server = axum::Server::from_tcp(listener)
        .unwrap()
        .serve(router.into_make_service());

    Ok(server)
}
