//! 共享词汇契约：释义模型与词性白名单。
//!
//! 只依赖 serde/strum/ts-rs，不引入数据库、HTTP 或平台相关代码，
//! 供单词书域、词典域与移动端复用。

#![forbid(unsafe_code)]

pub mod definition;
pub mod word_pos;
