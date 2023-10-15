//! src/startup.rs

use std::{net::TcpListener, sync::Arc};

use axum::{
    routing::{get, post, IntoMakeService},
    Router,
};
use hyper::server::conn::AddrIncoming;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tower::ServiceBuilder;

use tower_http::{
    request_id::{MakeRequestId, RequestId},
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
    ServiceBuilderExt,
};

use tracing::Level;
use uuid::Uuid;

type AppServer = axum::Server<AddrIncoming, IntoMakeService<Router>>;
pub struct Application {
    port: u16,
    server: AppServer,
}

impl Application {
    pub async fn build(configuration: Settings) -> hyper::Result<Self> {
        let connection_pool = get_connection_pool(&configuration.database);
        let sender_email = configuration
            .email_client
            .sender()
            .expect("Invalid sender email address");
        let timeout = configuration.email_client.timeout();
        let email_client = EmailClient::new(
            configuration.email_client.base_url,
            sender_email,
            configuration.email_client.authorization_token,
            timeout,
        );

        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );
        let listener = TcpListener::bind(address).expect("Failed to bind address");
        let port = listener.local_addr().unwrap().port();
        let server = run(listener, connection_pool, email_client)?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> hyper::Result<()> {
        self.server.await
    }
}

pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new().connect_lazy_with(configuration.with_db())
}

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

use crate::{
    configuration::{DatabaseSettings, Settings},
    email_client::EmailClient,
    routes::{health_check, subscribe},
};

fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
) -> hyper::Result<AppServer> {
    let email_client = Arc::new(email_client);

    Ok(axum::Server::from_tcp(listener)?.serve(
        axum::Router::new()
            .route("/health_check", get(health_check))
            .route("/subscriptions", post(subscribe))
            .with_state(db_pool.clone())
            .with_state(email_client.clone())
            .layer(
                ServiceBuilder::new()
                    .set_x_request_id(MakeRequestUuid)
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
