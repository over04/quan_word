//! 启动装配：日志、配置加载、数据库连接、路由与服务启动。

pub mod db;

use anyhow::Result;
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::router;

/// 启动 HTTP 服务：加载配置 → 初始化数据库 → 组装路由 → 监听。
pub async fn run() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let cfg = Config::load()?;
    tracing::info!("数据库: {}", cfg.database.url);

    let db = db::init_db(&cfg.database.url).await?;
    tracing::info!("数据库迁移完成");

    let app = router::build(db, cfg.server.auth_key.clone(), cfg.clone());
    let addr = format!("{}:{}", cfg.server.host, cfg.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("服务已启动: http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
