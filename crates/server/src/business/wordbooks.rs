//! 单词书业务域（`/api/wordbooks...`）：书册管理（列表 / 单书 / 创建 / 更新 / 删除）。
//!
//! 子资源 `words`（`/api/wordbooks/{id}/words...`）为嵌套子域，见 [`words`]。

pub mod dto;
mod error;
mod repo;
pub mod router;
mod service;
mod tags;
mod words;
