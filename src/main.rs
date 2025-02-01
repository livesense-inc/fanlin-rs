extern crate axum;
extern crate tokio;

use axum::{
    body::Body,
    extract::{ConnectInfo, OriginalUri, Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use clap::Parser;
use std::{net::SocketAddr, time::Duration};
use tokio::{net::TcpListener, signal};
use tower_http::{
    timeout::TimeoutLayer,
    trace::{DefaultOnResponse, TraceLayer},
    LatencyUnit,
};
use tracing_subscriber::{filter, prelude::*};

mod config;
mod handler;
mod infra;
mod query;

/// A web server to process and serve images
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path of a setting file
    #[arg(short, long, default_value_t = String::from("fanlin.json"))]
    conf: String,
}

#[tokio::main]
async fn main() {
    let logger = tracing_subscriber::fmt::layer().json();
    tracing_subscriber::registry()
        .with(logger.with_filter(filter::LevelFilter::INFO))
        .init();
    let args = Args::parse();
    let cfg = config::Config::from_file(args.conf).expect("failed to read a config file");
    let listen_addr = format!("{}:{}", &cfg.bind_addr, &cfg.port);
    let listener = TcpListener::bind(&listen_addr)
        .await
        .expect("failed to bind {listen_addr}");
    let cli = infra::Client::new(&cfg).await;
    let state = handler::State::new(cfg.providers.clone(), cli);
    // https://github.com/tower-rs/tower-http/blob/main/examples/axum-key-value-store/src/main.rs
    // https://docs.rs/tower-http/latest/tower_http/trace/index.html#on_request
    // https://docs.rs/tower-http/latest/tower_http/trace/struct.DefaultOnResponse.html
    let router = Router::new()
        .route("/ping", get(|| async { "pong" }))
        .fallback(generic_handler)
        .layer(TimeoutLayer::new(Duration::from_secs(10)))
        .layer(
            TraceLayer::new_for_http().on_response(
                DefaultOnResponse::new()
                    .level(tracing::Level::INFO)
                    .latency_unit(LatencyUnit::Millis),
            ),
        )
        .with_state(state);
    tracing::info!("serving on {listen_addr}");
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .expect("failed to start server");
}

async fn generic_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    OriginalUri(uri): OriginalUri,
    Query(params): Query<query::Query>,
    State(state): State<handler::State>,
) -> impl IntoResponse {
    tracing::info!(
        target: "tower_http::trace::on_request",
        message = "started processing request",
        client = ?addr,
        url = ?uri
    );
    if params.unsupported_scale_size() {
        return (
            StatusCode::BAD_REQUEST,
            [(header::CONTENT_TYPE, "text/plain")],
            Body::new("supported width and height: 20-2000 x 20-1000".to_string()),
        );
    }
    // https://docs.rs/axum/latest/axum/response/index.html
    let path = uri.path();
    let original = match state.get_image(path).await {
        Some(result) => match result {
            Ok(img) => img,
            Err(err) => {
                tracing::error!("failled to get an original image; {:?}", err);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    [(header::CONTENT_TYPE, "text/plain")],
                    Body::from("server error on fetching an image".to_string()),
                );
            }
        },
        None => {
            return (
                StatusCode::NOT_FOUND,
                [(header::CONTENT_TYPE, "text/plain")],
                Body::new("not found".to_string()),
            );
        }
    };
    // https://docs.rs/axum/latest/axum/body/struct.Body.html
    // https://github.com/tokio-rs/axum/blob/main/examples/stream-to-file/src/main.rs
    state.process_image(original, params).map_or_else(
        |err| {
            tracing::error!("failed to process an image; {:?}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "text/plain")],
                Body::from("server error on processing an image".to_string()),
            )
        },
        |(mime_type, processed)| {
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, mime_type)],
                Body::from(processed),
            )
        },
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
