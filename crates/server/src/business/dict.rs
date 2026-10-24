//! 词典业务域（`/api/dict...`）：多词典源抓词（源列表 / 聚合查询）。
//!
//! 与单词书域完全解耦：入参只有单词与源 id，出参是聚合后的词条与失败源列表。

mod breaker;
mod breaker_state;
mod cache;
mod cache_entry;
mod cache_key;
mod cached_lookup;
pub mod dto;
pub mod entry;
mod error;
pub mod example;
mod form_body;
mod form_field;
mod http;
mod http_header;
mod pos;
mod pos_form;
mod pos_mark;
mod pos_segment;
mod pos_token;
pub mod provider;
mod provider_error;
pub mod provider_id;
mod provider_outcome;
mod providers;
pub mod router;
mod service;
mod singleflight;
pub mod state;
mod suggest;
mod suggest_cache;
mod suggest_cache_entry;
mod suggest_item;
mod suggest_limiter;
mod suggest_payload;
mod suggest_wire;
mod text;
