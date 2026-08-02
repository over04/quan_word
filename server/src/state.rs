use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;
use sea_orm::DatabaseConnection;

use crate::dto::resp::wordbook_resp::WordbookResp;

/// 全局应用状态：共享数据库连接池 + 进程内存缓存。
///
/// 缓存均为单进程内存缓存，写操作同步失效保证一致性（见各 Service 的失效点）。
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<DatabaseConnection>,
    /// 单词书列表缓存（含 word_count）；None = 未缓存
    pub wordbooks_cache: Arc<Mutex<Option<Arc<Vec<WordbookResp>>>>>,
    /// random 打乱序列缓存：(book_id, seed) → 洗牌后的完整 id 序列；上限 8 条
    pub shuffle_cache: Arc<Mutex<HashMap<(i32, String), Vec<i32>>>>,
}
