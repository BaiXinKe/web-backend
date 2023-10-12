//! src/routes/subscriptions.rs

use axum::{
    async_trait,
    extract::{Form, FromRef, FromRequestParts, State},
    http::request::Parts,
    response::IntoResponse,
};
use chrono::Utc;
use hyper::StatusCode;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

struct DatabaseConnection(sqlx::pool::PoolConnection<sqlx::Postgres>);

#[async_trait]
impl<S> FromRequestParts<S> for DatabaseConnection
where
    PgPool: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(_parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let pool = PgPool::from_ref(state);
        let conn = pool.acquire().await.map_err(internal_error)?;
        Ok(Self(conn))
    }
}

#[derive(Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

pub async fn subscribe(
    State(pool): State<PgPool>,
    Form(form): Form<FormData>,
) -> impl IntoResponse {
    match sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    .execute(&pool)
    .await
    {
        Ok(_) => StatusCode::OK,
        Err(e) => {
            println!("Failed to execute query: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
