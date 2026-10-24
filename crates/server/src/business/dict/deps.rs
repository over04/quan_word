use std::sync::Arc;

use dict_core::transport::SearchTransport;

/// 词典域外部依赖：生产用 reqwest，测试注入 mock；移动端可换成平台传输层。
pub struct DictDeps {
    pub transport: Arc<dyn SearchTransport>,
}
