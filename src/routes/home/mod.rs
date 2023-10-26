//! src/routes/home/mod.rs

use axum::{http::header, response::IntoResponse};
use hyper::HeaderMap;

pub async fn home() -> impl IntoResponse {
    let mut header = HeaderMap::new();
    header.insert(header::CONTENT_TYPE, "text/html".parse().unwrap());

    (header, include_str!("home.html"))
}
