//! src/routes/login/get.rs
use axum::{
    extract::{Query, State},
    http::header,
    response::IntoResponse,
};
use hmac::{Hmac, Mac};
use hyper::StatusCode;
use secrecy::ExposeSecret;

use crate::startup::HmacSecret;

#[derive(serde::Deserialize)]
pub struct QueryParams {
    error: String,
    tag: String,
}

impl QueryParams {
    fn verfiy(self, secret: &HmacSecret) -> Result<String, anyhow::Error> {
        let tag = hex::decode(self.tag)?;
        let query_string = format!("error={}", urlencoding::Encoded::new(&self.error));

        let mut mac =
            Hmac::<sha2::Sha256>::new_from_slice(secret.0.expose_secret().as_bytes()).unwrap();
        mac.update(query_string.as_bytes());
        mac.verify_slice(&tag)?;

        Ok(self.error)
    }
}

pub async fn login_form(
    State(secret): State<HmacSecret>,
    Query(query): Query<Option<QueryParams>>,
) -> impl IntoResponse {
    let error_html = match query {
        None => "".into(),
        Some(query) => match query.verfiy(&secret) {
            Ok(error) => {
                format!("<p><i>{}</i></p>", htmlescape::encode_minimal(&error))
            }
            Err(e) => {
                tracing::warn!(error.message = %e, error.cause_chain=?e, "Failed to verify query parameters using the HMAC tag");
                "".into()
            }
        },
    };

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html")],
        format!(
            r#"
            <!DOCTYPE html>
            <html lang="en">
            <head>
                <meta http-equiv="content-type" content="text/html"; charset="utf-8">
                <title>Login</title>
            </head>
            <body>
                {error_html}
                <form action="/login" method="post">
                    <label>Username
                        <input
                            type="text"
                            placeholder="Enter Username"
                            name="username"
                        >
                    </label>
                    <label>Password
                        <input
                            type="password"
                            placeholder="Enter Password"
                            name="password"
                        >
                    </label>
                    <button type="submit">Login</button>
                </form>
            </body>
            </html>
            "#
        ),
    )
}
