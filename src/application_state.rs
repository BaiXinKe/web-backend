use axum::extract::FromRef;
use sqlx::PgPool;
use std::sync::Arc;

use crate::{email_client::EmailClient, startup::ApplicationBaseUrl};

#[derive(Clone)]
pub struct ApplicationState {
    pub db_pool: PgPool,
    pub email_client: EmailClientState,
    pub base_url: BaseUrlState,
}

impl ApplicationState {
    pub fn new(
        db_pool: PgPool,
        email_client: Arc<EmailClient>,
        base_url: Arc<ApplicationBaseUrl>,
    ) -> Self {
        Self {
            db_pool,
            email_client: EmailClientState::new(email_client),
            base_url: BaseUrlState::new(base_url),
        }
    }
}

impl FromRef<ApplicationState> for EmailClientState {
    fn from_ref(input: &ApplicationState) -> Self {
        input.email_client.clone()
    }
}

impl FromRef<ApplicationState> for PgPool {
    fn from_ref(input: &ApplicationState) -> Self {
        input.db_pool.clone()
    }
}

#[derive(Clone)]
pub struct EmailClientState(pub Arc<EmailClient>);

impl EmailClientState {
    pub fn new(email_client: Arc<EmailClient>) -> Self {
        Self(email_client)
    }
}

#[derive(Clone)]
pub struct BaseUrlState(pub Arc<ApplicationBaseUrl>);

impl BaseUrlState {
    pub fn new(base_url: Arc<ApplicationBaseUrl>) -> Self {
        Self(base_url)
    }
}
