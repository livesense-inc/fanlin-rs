extern crate axum;
extern crate tokio;

use axum::{
    body::Body,
    extract::{ConnectInfo, OriginalUri, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Router,
};
use chrono::prelude::*;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::{net::TcpListener, signal};
use tower_http::timeout::TimeoutLayer;

mod config;
mod handler;
mod infra;
mod query;

#[tokio::main]
async fn main() {
    let cfg = config::Config::new("fanlin.json").unwrap();
    let listener = TcpListener::bind(format!("{}:{}", &cfg.bind_addr, &cfg.port))
        .await
        .unwrap();
    let cli = infra::Client::new(&cfg).await;
    let state = Arc::new(handler::State::new(cfg.providers.clone(), cli));
    let router = Router::new()
        .route("/ping", get(|| async { "pong" }))
        .fallback(generic_handler)
        .layer(TimeoutLayer::new(Duration::from_secs(10)))
        .with_state(state.clone());
    println!(
        "{} Serving on {}:{}",
        Local::now(),
        &cfg.bind_addr,
        &cfg.port
    );
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .unwrap();
}

async fn generic_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    OriginalUri(uri): OriginalUri,
    Query(params): Query<query::Query>,
    State(state): State<Arc<handler::State>>,
) -> impl IntoResponse {
    println!("{} {} {}", Local::now(), addr, uri);
    // https://docs.rs/axum/latest/axum/response/index.html
    let path = uri.path();
    let original = match state.get_image(path).await {
        Some(result) => match result {
            Ok(img) => img,
            Err(err) => {
                eprintln!("failled to get an original image; {:?}", err);
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
    state.process_image(original, params).map_or_else(
        |err| {
            eprintln!("failed to process an image; {:?}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Body::from("server error".to_string()),
            )
        },
        |processed| (StatusCode::OK, Body::from(processed)),
    )
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
