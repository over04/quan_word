use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;
use sea_orm::DatabaseConnection;

use crate::business::wordbooks::dto::resp::WordbookResp;

/// 洗牌序列缓存上限（条）：超出后整体清空，避免缓存无限增长。
pub const SHUFFLE_CACHE_CAP: usize = 8;

/// random 打乱序列缓存条目：(book_id, seed) → 洗牌后的完整 id 序列
type ShuffleCache = Arc<Mutex<HashMap<(i32, String), Vec<i32>>>>;

/// 全局应用状态：共享数据库连接池 + 进程内存缓存。
///
/// 缓存均为单进程内存缓存，写操作同步失效保证一致性（见各 Service 的失效点）。
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<DatabaseConnection>,
    /// 访问密钥（config server.auth_key）；None = 未启用鉴权
    pub api_key: Option<Arc<str>>,
    /// 单词书列表缓存（含 word_count）；None = 未缓存
    pub wordbooks_cache: Arc<Mutex<Option<Arc<Vec<WordbookResp>>>>>,
    pub shuffle_cache: ShuffleCache,
}

impl AppState {
    /// 失效单词书列表缓存：单词书增删改、单词增删（影响 word_count）后调用。
    pub fn invalidate_wordbooks(&self) {
        *self.wordbooks_cache.lock() = None;
    }
}
