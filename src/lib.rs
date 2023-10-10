use std::net::TcpListener;

use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{get, IntoMakeService},
    Router,
};
use hyper::server::conn::AddrIncoming;

async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}

pub fn run(
    listener: TcpListener,
) -> std::result::Result<axum::Server<AddrIncoming, IntoMakeService<Router>>, std::io::Error> {
    let router = axum::Router::new().route("/health_check", get(health_check));

    let server = axum::Server::from_tcp(listener)
        .unwrap()
        .serve(router.into_make_service());

    Ok(server)
}
