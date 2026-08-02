use std::sync::Arc;

use sea_orm::DatabaseConnection;

/// 全局应用状态：共享数据库连接池。
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<DatabaseConnection>,
}
