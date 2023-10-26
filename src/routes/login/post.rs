//! src/routes/login/post.rs

use axum::extract::{Form, State};
use axum::response::{IntoResponse, Response};
use hmac::{Hmac, Mac};
use hyper::{header, StatusCode};
use secrecy::{Secret, ExposeSecret};
use sqlx::PgPool;

use crate::authentication::{Credientials, validate_credentials, AuthError};
use crate::routes::error_chain_fmt;
use crate::startup::HmacSecret;

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

#[tracing::instrument(
    skip(form, pool, secret), 
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    State(pool): State<PgPool>, 
    State(secret): State<HmacSecret>,
    Form(form): Form<FormData>
) -> Response {
    let credentials = Credientials {
        username: form.username,
        password: form.password
    };

    tracing::Span::current()
        .record("username", &tracing::field::display(&credentials.username));

   match validate_credentials(credentials, &pool).await {
    Ok(user_id) => {
        tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
        (
            StatusCode::SEE_OTHER,
            [(header::LOCATION, "/")]
        ).into_response()
    }
    Err(e) => {
        let e = match e {
            AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
            AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into())
        };
        let query_string = format!(
            "error={}", urlencoding::Encoded::new(e.to_string())
        );
        let hmac_tag = {
            let mut mac = Hmac::<sha2::Sha256>::new_from_slice(
                secret.0.expose_secret().as_bytes()
            ).unwrap();
            mac.update(query_string.as_bytes());
            mac.finalize().into_bytes()
        };

        (
            StatusCode::SEE_OTHER,
            [(header::LOCATION, & format!("/login?{query_string}&tag={:x}", hmac_tag))],
        ).into_response()
    }
   }

}


#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error)
}


impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}


