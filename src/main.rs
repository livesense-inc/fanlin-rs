extern crate axum;
extern crate tokio;

use axum::{
    body::Body,
    extract::{OriginalUri, Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::get,
};
use clap::Parser;
use tracing_subscriber::prelude::*;

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

    /// JSON data for setting
    #[arg(short, long)]
    json: Option<String>,
}

#[tokio::main]
async fn main() {
    // https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/struct.SubscriberBuilder.html
    let logger = tracing_subscriber::fmt::layer()
        .with_target(false)
        .json()
        .with_span_list(false)
        .flatten_event(true);
    tracing_subscriber::registry()
        .with(logger.with_filter(tracing_subscriber::filter::LevelFilter::INFO))
        .init();
    let args = Args::parse();
    let cfg = match args.json {
        Some(j) => config::Config::from_reader(j.as_bytes()).expect("failed to read JSON"),
        None => config::Config::from_file(args.conf).expect("failed to read a config file"),
    };
    let listen_addr = format!("{}:{}", &cfg.bind_addr, &cfg.port);
    let listener = tokio::net::TcpListener::bind(&listen_addr)
        .await
        .expect("failed to bind address");
    let cli = infra::Client::new(&cfg).await;
    let mut state = handler::State::new(cfg.providers.clone(), cli);
    if let Some(p) = cfg.fallback_path {
        state.with_fallback(p.as_str()).await.map_or_else(
            |err| {
                tracing::warn!("failed to initialize fallback image; {err:?}");
            },
            |_| {},
        )
    };
    // https://github.com/tower-rs/tower-http/blob/main/examples/axum-key-value-store/src/main.rs
    // https://docs.rs/axum/latest/axum/middleware/index.html
    // https://docs.rs/tower-http/latest/tower_http/trace/index.html
    // https://docs.rs/tower-http/latest/tower_http/timeout/struct.TimeoutLayer.html
    // https://github.com/tower-rs/tower-http/issues/296
    // https://docs.rs/tracing/latest/tracing/span/struct.Span.html
    let router = axum::Router::new()
        .route("/ping", get(|| async { "pong" }))
        .fallback(generic_handler)
        .layer(
            tower::ServiceBuilder::new()
                .layer(
                    tower_http::trace::TraceLayer::new_for_http()
                        .make_span_with(
                            tower_http::trace::DefaultMakeSpan::new().level(tracing::Level::INFO),
                        )
                        .on_response(
                            tower_http::trace::DefaultOnResponse::new()
                                .level(tracing::Level::INFO)
                                .latency_unit(tower_http::LatencyUnit::Millis),
                        )
                        .on_failure(()),
                )
                .layer(tower_http::timeout::TimeoutLayer::new(
                    std::time::Duration::from_secs(10),
                ))
                .layer(tower::limit::concurrency::ConcurrencyLimitLayer::new(
                    cfg.max_clients,
                )),
        )
        .with_state(std::sync::Arc::new(state));
    tracing::info!("serving on {listen_addr}");
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("failed to start server");
}

const CONTENT_TYPE_TEXT_PLAIN: (header::HeaderName, &str) = (header::CONTENT_TYPE, "text/plain");

#[axum::debug_handler]
async fn generic_handler(
    OriginalUri(uri): OriginalUri,
    Query(params): Query<query::Query>,
    State(state): State<std::sync::Arc<handler::State>>,
) -> impl IntoResponse {
    if params.unsupported_scale_size() {
        return (
            StatusCode::BAD_REQUEST,
            [CONTENT_TYPE_TEXT_PLAIN],
            Body::from(format!(
                "supported width and height: {}",
                query::size_range_info()
            )),
        );
    }
    // https://docs.rs/axum/latest/axum/response/index.html
    let path = uri.path();
    let original = match state.get_image(path).await {
        Ok(option) => match option {
            Some(img) => img,
            None => {
                return fallback_or_message(&state, &params, StatusCode::NOT_FOUND, "not found");
            }
        },
        Err(err) => {
            tracing::error!("failled to get an original image; {err:?}");
            return fallback_or_message(
                &state,
                &params,
                StatusCode::INTERNAL_SERVER_ERROR,
                "server error on fetching an image",
            );
        }
    };
    // https://docs.rs/axum/latest/axum/body/struct.Body.html
    // https://github.com/tokio-rs/axum/blob/main/examples/stream-to-file/src/main.rs
    state.process_image(&original, &params).map_or_else(
        |err| {
            tracing::error!("failed to process an image; {err:?}");
            fallback_or_message(
                &state,
                &params,
                StatusCode::INTERNAL_SERVER_ERROR,
                "server error on processing an image",
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

fn fallback_or_message(
    state: &handler::State,
    params: &query::Query,
    status: StatusCode,
    message: &'static str,
) -> (StatusCode, [(header::HeaderName, &'static str); 1], Body) {
    state.fallback(params).map_or_else(
        |_err| (status, [CONTENT_TYPE_TEXT_PLAIN], Body::from(message)),
        |(mime_type, processed)| {
            (
                status,
                [(header::CONTENT_TYPE, mime_type)],
                Body::from(processed),
            )
        },
    )
}

async fn shutdown_signal() {
    // https://github.com/tokio-rs/axum/blob/main/examples/graceful-shutdown/src/main.rs
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[tokio::test]
async fn test_generic_handler() {
    let client = infra::Client::for_test().await;
    let mut bucket_manager = infra::s3::BucketManager::new(client.s3.clone());
    let bucket = bucket_manager
        .create()
        .await
        .expect("failed to create a bucket");
    bucket_manager
        .upload_fixture_files(&bucket, "images", "images")
        .await
        .expect("failed to upload fixture files");
    let (port, mock_server) = infra::web::run_mock_server("/images", "images").await;
    let providers = Vec::from([
        config::Provider {
            path: "foo".to_string(),
            src: format!("s3://{bucket}/images"),
        },
        config::Provider {
            path: "bar".to_string(),
            src: format!("http://127.0.0.1:{port}/images"),
        },
        config::Provider {
            path: "baz".to_string(),
            src: "file://localhost/./images".to_string(),
        },
    ]);
    let state = std::sync::Arc::new(handler::State::new(providers, client));
    struct Case {
        url: &'static str,
        want_status: StatusCode,
        want_type: &'static str,
    }
    let cases = [
        Case {
            url: "http://127.0.0.1:3000/foo/lenna.jpg",
            want_status: StatusCode::OK,
            want_type: "image/jpeg",
        },
        Case {
            url: "http://127.0.0.1:3000/foo/lenna.jpg?w=300&h=200",
            want_status: StatusCode::OK,
            want_type: "image/jpeg",
        },
        Case {
            url: "http://127.0.0.1:3000/foo/lenna.jpg?w=300&h=200&avif=true",
            want_status: StatusCode::OK,
            want_type: "image/avif",
        },
        Case {
            url: "http://127.0.0.1:3000/foo/lenna.jpg?w=300&h=200&webp=true",
            want_status: StatusCode::OK,
            want_type: "image/webp",
        },
        Case {
            url: "http://127.0.0.1:3000/foo/lenna.jpg?w=9999&h=9999",
            want_status: StatusCode::BAD_REQUEST,
            want_type: "text/plain",
        },
        Case {
            url: "http://127.0.0.1:3000/foo/lenna.png",
            want_status: StatusCode::OK,
            want_type: "image/png",
        },
        Case {
            url: "http://127.0.0.1:3000/foo/lenna.png?w=300&h=200&avif=true",
            want_status: StatusCode::OK,
            want_type: "image/avif",
        },
        Case {
            url: "http://127.0.0.1:3000/foo/lenna.gif",
            want_status: StatusCode::OK,
            want_type: "image/gif",
        },
        Case {
            url: "http://127.0.0.1:3000/foo/lenna.gif?w=300&h=200&webp=true",
            want_status: StatusCode::OK,
            want_type: "image/webp",
        },
        Case {
            url: "http://127.0.0.1:3000/foo/lenna.txt",
            want_status: StatusCode::INTERNAL_SERVER_ERROR,
            want_type: "text/plain",
        },
        Case {
            url: "http://127.0.0.1:3000/foo/who.jpg",
            want_status: StatusCode::NOT_FOUND,
            want_type: "text/plain",
        },
        Case {
            url: "http://127.0.0.1:3000/bar/lenna.jpg",
            want_status: StatusCode::OK,
            want_type: "image/jpeg",
        },
        Case {
            url: "http://127.0.0.1:3000/bar/who.jpg",
            want_status: StatusCode::NOT_FOUND,
            want_type: "text/plain",
        },
        Case {
            url: "http://127.0.0.1:3000/baz/lenna.jpg",
            want_status: StatusCode::OK,
            want_type: "image/jpeg",
        },
        Case {
            url: "http://127.0.0.1:3000/baz/who.jpg",
            want_status: StatusCode::NOT_FOUND,
            want_type: "text/plain",
        },
    ];
    for c in cases {
        let uri = c
            .url
            .parse::<axum::http::Uri>()
            .expect("failed to parse a string as an URI");
        let query: Query<query::Query> =
            axum::extract::Query::try_from_uri(&uri).expect("failed to parse query from URI");
        let got = generic_handler(OriginalUri(uri), query, State(state.clone()))
            .await
            .into_response();
        assert_eq!(
            got.status(),
            c.want_status,
            "case: {}, bucket: {bucket}",
            c.url
        );
        assert_eq!(
            got.headers().get(header::CONTENT_TYPE).unwrap(),
            c.want_type,
            "case: {}, bucket: {bucket}",
            c.url
        );
    }
    bucket_manager.clean().await.unwrap();
    mock_server.abort();
}
