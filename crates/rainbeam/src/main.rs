//! 🌈 Rainbeam!
#![doc = include_str!("../../../README.md")]
#![doc(issue_tracker_base_url = "https://github.com/swmff/rainbeam/issues")]
#![doc(html_favicon_url = "https://rainbeam.net/static/favicon.svg")]
#![doc(html_logo_url = "https://rainbeam.net/static/favicon.svg")]
use axum::routing::{get, get_service};
use axum::Router;

use tower_http::trace::{self, TraceLayer};
use tracing::{info, Level};

use authbeam::{api as AuthApi, Database as AuthDatabase};
use databeam::config::Config as DataConf;

mod config;
mod database;
mod model;
mod routing;

/// Main server process
#[tokio::main]
pub async fn main() {
    let mut config = config::Config::get_config();

    let home = std::env::var("HOME").expect("failed to read $HOME");
    let static_dir = format!("{home}/.config/xsu-apps/rainbeam/static");
    config.static_dir = static_dir.clone();

    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    // create databases
    let auth_database = AuthDatabase::new(
        DataConf::get_config().connection, // pull connection config from config file
        authbeam::ServerOptions {
            captcha: config.captcha.clone(),
            registration_enabled: config.registration_enabled,
            real_ip_header: config.real_ip_header.clone(),
        },
    )
    .await;
    auth_database.init().await;

    let database = database::Database::new(
        DataConf::get_config().connection,
        auth_database.clone(),
        config.clone(),
    )
    .await;
    database.init().await;

    if config.migration == true {
        // database.migrate_ghsa_gc85_x5qp_77qq().await.unwrap(); // MIGRATION: c8f94a27b1ec3ef171cdc0b8bcf57b8af034e31b
        std::process::exit(0);
    }

    // create app
    let app = Router::new()
        // api
        .nest_service("/api/auth", AuthApi::routes(auth_database.clone()))
        .nest("/api/util", routing::api::util::routes(database.clone()))
        .nest("/api/v1", routing::api::routes(database.clone()))
        // pages
        .nest("/", routing::pages::routes(database.clone()).await)
        // ...
        .nest_service(
            "/static",
            get_service(tower_http::services::ServeDir::new(&static_dir)),
        )
        .nest_service(
            "/manifest.json",
            get_service(tower_http::services::ServeFile::new(format!(
                "{static_dir}/manifest.json"
            ))),
        )
        .fallback_service(get(routing::pages::not_found).with_state(database.clone()))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
        );

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", config.port))
        .await
        .unwrap();

    info!("🌈 Starting server at: http://localhost:{}!", config.port);
    axum::serve(listener, app).await.unwrap();
}