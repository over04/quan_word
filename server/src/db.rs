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
        // WAL：读写并发不互斥（翻页预取与写操作同时进行时不锁库）
        db.execute_unprepared("PRAGMA journal_mode = WAL").await?;
        // WAL 下崩溃安全性与性能的平衡点
        db.execute_unprepared("PRAGMA synchronous = NORMAL").await?;
        // 并发写等待 5s 而非立即报 SQLITE_BUSY
        db.execute_unprepared("PRAGMA busy_timeout = 5000").await?;
        // 页缓存 20MB（默认约 2MB）
        db.execute_unprepared("PRAGMA cache_size = -20000").await?;
    }
    migration::Migrator::up(&db, None).await?;
    Ok(db)
}
