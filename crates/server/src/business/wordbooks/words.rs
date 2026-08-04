//! 单词子域（`/api/wordbooks/{id}/words...` 与 `/api/words/{id}`）：分页浏览 / 搜索 / 增删改。

pub mod dto;
mod error;
mod file_type;
mod import;
mod import_filter;
mod order;
mod pos;
mod repo;
pub mod router;
mod service;
mod sort;
mod sort_dir;
pub mod tag_match;
mod template_format;
