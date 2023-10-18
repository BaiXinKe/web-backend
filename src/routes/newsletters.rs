//! src/routes/newletters.rs

use anyhow::{anyhow, Context};
use axum::extract::{Json, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use sqlx::PgPool;

use crate::application_state::ApplicationState;
use crate::domain::SubscriberEmail;
use crate::email_client;

use super::error_chain_fmt;

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for PublishError {
    fn into_response(self) -> axum::response::Response {
        match self {
            PublishError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
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

pub async fn publish_newsletter(
    State(app_state): State<ApplicationState>,
    Json(body): Json<BodyData>,
) -> Result<Response, PublishError> {
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
