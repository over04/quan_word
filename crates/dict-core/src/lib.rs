//! 词典搜索内核：平台无关的模型、插件包解析、配方引擎、注册表与传输边界。
//!
//! 不依赖 `reqwest` / `tokio` / 文件系统 / 网络实现：宿主提供 `SearchTransport`，
//! 插件字节由宿主传入，因此可编译到 iOS / Android / wasm 等任意 Rust 目标。

#![forbid(unsafe_code)]

pub mod capability;
pub mod engine_limits;
pub mod entry;
pub mod example;
pub mod form_field_spec;
pub mod header_spec;
pub mod plugin_engine;
pub mod plugin_error;
pub mod plugin_info;
pub mod plugin_limits;
pub mod plugin_load_report;
pub mod plugin_manifest;
pub mod plugin_package;
pub mod plugin_registry;
pub mod plugin_source;
pub mod plugin_status;
pub mod pos;
pub mod pos_form;
pub mod pos_mark;
pub mod pos_segment;
pub mod pos_token;
pub mod provider_error;
pub mod provider_id;
pub mod recipe;
pub mod registry_entry;
pub mod reload_report;
pub mod request_method;
pub mod search_engine;
pub mod search_request;
pub mod search_response;
pub mod suggestion;
pub mod text;
pub mod transport;
pub mod transport_error;
