//! 词典业务域（`/api/dict...`）：插件化多源抓词（源列表 / 插件管理 / 聚合查询）。
//!
//! 与单词书域完全解耦：入参只有单词与源 id，出参是聚合后的词条与失败源列表。

mod breaker;
mod breaker_state;
mod cache;
mod cache_entry;
mod cache_key;
mod cached_lookup;
pub mod deps;
pub mod dto;
mod error;
mod plugin_prefs;
mod plugin_sources;
mod plugin_store;
mod provider_outcome;
pub mod router;
mod service;
mod singleflight;
pub mod state;
mod suggest;
mod suggest_cache;
mod suggest_cache_entry;
mod suggest_limiter;
