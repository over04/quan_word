mod config;
mod controller;
mod db;
mod dto;
mod error;
mod model;
mod router;
mod service;
mod state;

use anyhow::Result;
use tracing_subscriber::EnvFilter;

use crate::config::config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let cfg = Config::load()?;
    tracing::info!("数据库: {}", cfg.database.url);

    let db = db::init(&cfg.database.url).await?;
    tracing::info!("数据库迁移完成");

    let app = router::build(db);
    let addr = format!("{}:{}", cfg.server.host, cfg.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("服务已启动: http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
