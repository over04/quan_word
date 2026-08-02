use anyhow::Result;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;

/// 建立数据库连接并执行迁移。
///
/// SQLite 特殊处理：
/// - 自动创建 `data/` 目录（首次启动时数据库文件不存在）
/// - 显式开启外键约束（SQLite 默认关闭，不开启则删除单词书不会级联删除单词）
pub async fn init(url: &str) -> Result<DatabaseConnection> {
    if url.starts_with("sqlite:") {
        std::fs::create_dir_all("data")?;
    }
    let db = Database::connect(url).await?;
    if url.starts_with("sqlite:") {
        db.execute_unprepared("PRAGMA foreign_keys = ON").await?;
    }
    migration::Migrator::up(&db, None).await?;
    Ok(db)
}
