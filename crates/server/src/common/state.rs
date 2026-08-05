use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use parking_lot::Mutex;
use sea_orm::DatabaseConnection;

use crate::business::wordbooks::dto::resp::WordbookResp;
use crate::business::wordbooks::words::dto::import::ImportRowData;
use crate::business::wordbooks::words::tag_group::TagGroup;
use crate::business::wordbooks::words::tag_match::TagMatch;
use crate::config::Config;

/// 洗牌序列缓存上限（条）：超出后整体清空，避免缓存无限增长。
pub const SHUFFLE_CACHE_CAP: usize = 8;

/// random 打乱序列缓存条目：(book_id, 筛选组, 组间连接词, seed) → 洗牌后的完整 id 序列
/// （groups 内 ids 已排序去重；空 = 不筛选；匹配模式与连接词直接用业务枚举 `TagMatch`，不做字符串降级）
type ShuffleCache = Arc<Mutex<HashMap<(i32, Vec<TagGroup>, Vec<TagMatch>, String), Vec<i32>>>>;

/// 导入预览会话条目：token → (book_id, 全量行数据)。
/// rows 为 typed `ImportRowData`（Arc 共享避免拷贝；跨请求边界的缓存不中继序列化字节）。
pub struct ImportCacheEntry {
    pub book_id: i32,
    pub rows: Arc<Vec<ImportRowData>>,
    pub created_at: Instant,
}

pub type ImportCache = Arc<Mutex<HashMap<String, ImportCacheEntry>>>;

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
    /// 导入预览会话缓存：token → 解析结果（TTL/容量由 config.import 控制）
    pub import_cache: ImportCache,
    /// 应用配置（含 import 段的 TTL/容量/行数上限）
    pub config: Config,
}

impl AppState {
    /// 失效单词书列表缓存：单词书增删改、单词增删（影响 word_count）后调用。
    pub fn invalidate_wordbooks(&self) {
        *self.wordbooks_cache.lock() = None;
    }

    /// 启动导入预览会话的后台清理：按配置间隔清除过期条目。
    /// 服务启动时调用一次；任务随进程结束而终止（幂等：调用方保证只调一次）。
    pub fn spawn_import_cache_cleaner(&self) {
        let cache = Arc::clone(&self.import_cache);
        let interval = std::time::Duration::from_secs(self.config.import.cache_cleanup_secs);
        let ttl = std::time::Duration::from_secs(self.config.import.cache_ttl_secs);
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                cache.lock().retain(|_, e| e.created_at.elapsed() < ttl);
            }
        });
    }
}
