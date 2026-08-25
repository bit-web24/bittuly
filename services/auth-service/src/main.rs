use auth_service::*;

use metrics_exporter_prometheus::PrometheusBuilder;
use shared::config;
use shared::postgres;
use std::sync::Arc;
use tower::Layer;
use tower_http::cors::CorsLayer;
use tower_http::normalize_path::NormalizePathLayer;
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() {
    shared::telemetry::init_tracing("auth-service");

    let settings = config::AuthConfig::from_env().expect("Failed to load setting from environment");
    let amqp_url = std::env::var("RABBITMQ_URL").expect("RABBITMQ_URL must be set");

    let db = postgres::init_pg_pool(&settings.database_url)
        .await
        .expect("Failed to connect to primary database");
    let read_db = postgres::init_pg_pool(&settings.database_read_url)
        .await
        .expect("Failed to connect to read-replica database");

    let rabbitmq = shared::rabbitmq::init_rabbitmq_pool(&amqp_url)
        .await
        .expect("Failed to create RabbitMQ pool");

    let prometheus_handle = PrometheusBuilder::new()
        .install_recorder()
        .expect("Failed to install Prometheus recorder");

    let pg_repo = Arc::new(repository::PgUserRepo(db));
    let read_pg_repo = Arc::new(repository::PgUserRepo(read_db));
    let hasher = Arc::new(password_hasher::BcryptHasher::from_mode(&settings.mode));
    let publisher = Arc::new(shared::rabbitmq::RabbitMqPublisher::new(rabbitmq));
    let auth_state = Arc::new(models::AuthState::new(
        pg_repo,
        read_pg_repo,
        publisher,
        hasher,
        prometheus_handle,
    ));

    let cors = CorsLayer::new()
        .allow_origin(
            settings
                .cors_origin
                .parse::<axum::http::HeaderValue>()
                .expect("Invalid CORS origin"),
        )
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
        ])
        .allow_credentials(true);

    use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};

    let app = axum::Router::new()
        .nest("/api/auth", routes::auth_routes())
        .layer(OtelInResponseLayer) // adds trace-id to response headers
        .layer(OtelAxumLayer::default()) // extracts W3C traceparent from request
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(auth_state);

    let listener =
        tokio::net::TcpListener::bind(format!("{}:{}", settings.server_addr, settings.server_port))
            .await
            .expect("failed to bind listener");

    tracing::info!(
        "Auth service listening on {}:{} [mode={} cors={}]",
        settings.server_addr,
        settings.server_port,
        settings.mode,
        settings.cors_origin
    );

    // NormalizePathLayer strips trailing slashes so /api/auth/ matches /api/auth
    let app = NormalizePathLayer::trim_trailing_slash().layer(app);
    let app: axum::routing::IntoMakeService<_> =
        axum::ServiceExt::<axum::extract::Request>::into_make_service(app);

    let server = axum::serve(listener, app);
    let graceful = server.with_graceful_shutdown(shutdown_signal());

    if let Err(err) = graceful.await {
        tracing::error!("Auth server error: {err}");
    }

    // Ensure all traces are flushed before exiting
    shared::telemetry::shutdown_tracing();
}

async fn shutdown_signal() {
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

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("Shutdown signal received, starting graceful shutdown");
}
