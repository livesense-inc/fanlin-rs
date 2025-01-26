extern crate axum;
extern crate tokio;

use axum::{
    body::Body,
    extract::{OriginalUri, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Router,
};
use chrono::prelude::*;
use std::{sync::Arc, time::Duration};
use tokio::{net::TcpListener, signal};
use tower_http::timeout::TimeoutLayer;

mod config;
mod handler;
mod infra;
mod query;

#[tokio::main]
async fn main() {
    let cfg = config::Config::new();

    let listener = TcpListener::bind(format!("{}:{}", &cfg.bind_addr, &cfg.port))
        .await
        .unwrap();

    let cli = infra::Client::new(&cfg).await;
    let state = Arc::new(handler::State::new(cfg.clone(), cli));
    let router = Router::new()
        .route("/ping", get(|| async { "pong" }))
        .fallback(generic_handler)
        .layer(TimeoutLayer::new(Duration::from_secs(10)))
        .with_state(state.clone());

    println!("{} Serving on {}", Local::now(), state.root_uri);
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn generic_handler(
    OriginalUri(uri): OriginalUri,
    Query(params): Query<query::Query>,
    State(state): State<Arc<handler::State>>,
) -> impl IntoResponse {
    println!("{} {}{}", Local::now(), state.root_uri, uri);
    // https://docs.rs/axum/latest/axum/response/index.html
    let path = uri.path();
    let original = match state.get_image(path).await {
        Some(result) => match result {
            Ok(img) => img,
            Err(err) => {
                eprintln!("failled to get original image; {:?}", err);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Body::from("server error".to_string()),
                );
            }
        },
        None => {
            return (StatusCode::NOT_FOUND, Body::new("not found".to_string()));
        }
    };
    // https://docs.rs/axum/latest/axum/body/struct.Body.html
    // https://github.com/tokio-rs/axum/blob/main/examples/stream-to-file/src/main.rs
    match state.process_image(original, params) {
        Ok(processed) => (StatusCode::OK, Body::from(processed)),
        Err(err) => {
            eprintln!("failed to process image; {:?}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Body::from("server error".to_string()),
            )
        }
    }
}

async fn shutdown_signal() {
    // https://github.com/tokio-rs/axum/blob/main/examples/graceful-shutdown/src/main.rs
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
