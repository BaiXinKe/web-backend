//! src/startup.rs

use std::net::TcpListener;

use axum::{
    routing::{get, post, IntoMakeService},
    Router,
};
use hyper::server::conn::AddrIncoming;
use sqlx::PgPool;
use tower::ServiceBuilder;

use tower_http::{
    request_id::{MakeRequestId, RequestId},
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
    ServiceBuilderExt,
};

use tracing::Level;
use uuid::Uuid;

#[derive(Clone, Default)]
struct MakeRequestUuid;

impl MakeRequestId for MakeRequestUuid {
    fn make_request_id<B>(
        &mut self,
        _request: &hyper::Request<B>,
    ) -> Option<tower_http::request_id::RequestId> {
        let request_id = Uuid::new_v4().to_string();
        Some(RequestId::new(request_id.parse().unwrap()))
    }
}

use crate::routes::{health_check, subscribe};

type App = axum::Server<AddrIncoming, IntoMakeService<Router>>;

pub fn run(listener: TcpListener, db_pool: PgPool) -> hyper::Result<App> {
    Ok(axum::Server::from_tcp(listener)?.serve(
        axum::Router::new()
            .route("/health_check", get(health_check))
            .route("/subscriptions", post(subscribe))
            .with_state(db_pool.clone())
            .layer(
                ServiceBuilder::new()
                    .set_x_request_id(MakeRequestUuid::default())
                    .layer(
                        TraceLayer::new_for_http()
                            .make_span_with(
                                DefaultMakeSpan::new()
                                    .include_headers(true)
                                    .level(Level::INFO),
                            )
                            .on_response(DefaultOnResponse::new().include_headers(true)),
                    )
                    .propagate_x_request_id(),
            )
            .into_make_service(),
    ))
}
