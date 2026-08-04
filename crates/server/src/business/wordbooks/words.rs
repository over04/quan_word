//! 单词子域（`/api/wordbooks/{id}/words...` 与 `/api/words/{id}`）：分页浏览 / 搜索 / 增删改。

pub mod dto;
mod error;
mod import;
mod order;
mod repo;
pub mod router;
mod service;
mod sort;
mod sort_dir;
mod tag_match;
