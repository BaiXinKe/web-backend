//! src/routes/newletters.rs

use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::extract::{Json, State};
use axum::http::{self, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use base64::Engine;
use hyper::header;
use secrecy::ExposeSecret;
use secrecy::Secret;
use sqlx::PgPool;

use crate::application_state::ApplicationState;
use crate::domain::SubscriberEmail;

use super::error_chain_fmt;

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for PublishError {
    fn into_response(self) -> axum::response::Response {
        match self {
            PublishError::UnexpectedError(error) => {
                tracing::error!("Unexpected error caused by {}", error);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
            PublishError::AuthError(_) => {
                let mut response = StatusCode::UNAUTHORIZED.into_response();
                response.headers_mut().insert(
                    header::WWW_AUTHENTICATE,
                    http::HeaderValue::from_str(r#"Basic realm="publish""#).unwrap(),
                );
                response
            }
        }
    }
}

#[derive(serde::Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(serde::Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

struct Credientials {
    username: String,
    password: Secret<String>,
}

#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip(app_state, header_map, body),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn publish_newsletter(
    State(app_state): State<ApplicationState>,
    header_map: HeaderMap,
    Json(body): Json<BodyData>,
) -> Result<Response, PublishError> {
    let credentials = basic_authentication(&header_map).map_err(PublishError::AuthError)?;
    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));

    let user_id = validate_credentials(credentials, &app_state.db_pool).await?;
    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

    let susbcribers = get_confirmed_subscribers(&app_state.db_pool).await?;
    for subscriber in susbcribers {
        match subscriber {
            Ok(subscriber) => app_state
                .email_client
                .0
                .send_email(
                    &subscriber.email,
                    &body.title,
                    &body.content.html,
                    &body.content.text,
                )
                .await
                .with_context(|| {
                    format!("Failed to send newsletter issue to {}", subscriber.email)
                })?,
            Err(error) => {
                tracing::warn!(
                    error.cause_chain = ?error,
                    "Skipping a confirmed susbcriber. \
                    Their stored contact details are invalid"
                )
            }
        }
    }

    Ok(StatusCode::OK.into_response())
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[tracing::instrument(name = "Get confirmed subscribers", skip(pool))]
async fn get_confirmed_subscribers(
    pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let confirmed_susbcribers = sqlx::query!(
        r#"
        SELECT email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| match SubscriberEmail::parse(r.email) {
        Ok(email) => Ok(ConfirmedSubscriber { email }),
        Err(error) => Err(anyhow::anyhow!(error)),
    })
    .collect();

    Ok(confirmed_susbcribers)
}

fn basic_authentication(headers: &HeaderMap) -> Result<Credientials, anyhow::Error> {
    let header_value = headers
        .get("Authorization")
        .context("The 'Authorization' header was missing")?
        .to_str()
        .context("The 'Authorization' header was not a valid UTF8 string.")?;
    let base64encoded_segment = header_value
        .strip_prefix("Basic ")
        .context("The authorization scheme was not 'Basic'.")?;
    let decoded_bytes = base64::engine::general_purpose::STANDARD
        .decode(base64encoded_segment)
        .context("Failed to base64-decode 'Basic' credentials.")?;
    let decoded_credentials = String::from_utf8(decoded_bytes)
        .context("The decoded credential string is not valid UTF8.")?;

    let mut credientials = decoded_credentials.splitn(2, ':');
    let username = credientials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A username must be provided in 'Basic' auth."))?
        .to_string();
    let password = credientials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A password must be provided in 'Basic' auth."))?
        .to_string();

    Ok(Credientials {
        username,
        password: Secret::new(password),
    })
}

async fn validate_credentials(
    credential: Credientials,
    pool: &PgPool,
) -> Result<uuid::Uuid, PublishError> {
    let row: Option<_> = sqlx::query!(
        r#"
        SELECT user_id, password_hash
        FROM users
        WHERE username = $1
        "#,
        credential.username
    )
    .fetch_optional(pool)
    .await
    .context("Failed to perform a query to retrieve stored credentials.")
    .map_err(PublishError::UnexpectedError)?;

    let (expected_password_hash, user_id) = match row {
        Some(row) => (row.password_hash, row.user_id),
        None => {
            return Err(PublishError::AuthError(anyhow::anyhow!(
                "Unknown username."
            )))
        }
    };

    let expected_password_hash = PasswordHash::new(&expected_password_hash)
        .context("Failed to parse hash in PHC string format.")
        .map_err(PublishError::UnexpectedError)?;

    Argon2::default()
        .verify_password(
            credential.password.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .context("Invalid password.")
        .map_err(PublishError::AuthError)?;

    Ok(user_id)
}
