//! src/routes/login/post.rs

use axum::extract::{Form, State};
use axum::response::{IntoResponse, Response};
use hyper::{header, StatusCode};
use secrecy::Secret;
use sqlx::PgPool;

use crate::authentication::{validate_credentials, AuthError, Credientials};
use crate::routes::error_chain_fmt;

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

#[tracing::instrument(
    skip(form, pool),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(State(pool): State<PgPool>, Form(form): Form<FormData>) -> Response {
    let credentials = Credientials {
        username: form.username,
        password: form.password,
    };

    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));

    match validate_credentials(credentials, &pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
            (StatusCode::SEE_OTHER, [(header::LOCATION, "/")]).into_response()
        }
        Err(e) => {
            let e = match e {
                AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
                AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()),
            };
            (
                StatusCode::SEE_OTHER,
                [
                    (header::LOCATION, &"/login".to_string()),
                    (header::SET_COOKIE, &format!("_flash={}", e)),
                ],
            )
                .into_response()
        }
    }
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}
