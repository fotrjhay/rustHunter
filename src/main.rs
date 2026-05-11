mod config;
mod errors;
mod models;
mod routes;
mod services;
mod utils;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use tower_http::trace::TraceLayer;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::config::AppConfig;
use crate::routes::create_router;
use crate::services::browser::BrowserService;
use crate::services::job_search::JobSearchService;

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub job_search: Arc<JobSearchService>,
    pub browser: Arc<BrowserService>,
}

#[tokio::main]
async fn main() {
    init_tracing();

    if let Err(err) = run().await {
        error!(error = %err, "service failed");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::from_env()?;

    if matches!(std::env::args().nth(1).as_deref(), Some("login")) {
        log_config_summary(&config, "login");
        BrowserService::new(config).manual_login().await?;
        return Ok(());
    }

    let addr = SocketAddr::from((config.server_host, config.server_port));
    log_config_summary(&config, "api");

    let state = AppState {
        config: config.clone(),
        job_search: Arc::new(JobSearchService::new(config.clone())),
        browser: Arc::new(BrowserService::new(config)),
    };

    let app: Router = create_router(state).layer(TraceLayer::new_for_http());
    let listener = tokio::net::TcpListener::bind(addr).await?;

    info!(%addr, "rusthunter api listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("rusthunter=debug,tower_http=info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}

fn log_config_summary(config: &AppConfig, mode: &str) {
    info!(
        mode,
        headless = config.browser_headless,
        driver_url = %config.browser_driver_url,
        profile_dir = %config.browser_profile_dir,
        area_code = config.area_code,
        items_per_page = config.items_per_page,
        page_timeout_ms = config.page_timeout,
        hh_locale = %config.hh_locale,
        server_host = %config.server_host,
        server_port = config.server_port,
        "configuration loaded"
    );
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    info!("shutdown signal received");
}
