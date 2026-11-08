use hyper::StatusCode;
use hyperoute_core::IntoErrorResponse;

/// 业务错误，handler返回Err时转换成纯文本错误响应
#[derive(Debug)]
pub struct BizError {
    status: StatusCode,
    body: String,
}

impl BizError {
    /// 业务错误消息，状态码为500
    pub fn msg(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, msg)
    }

    /// 请求参数错误，状态码为400
    pub fn param(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, msg)
    }

    /// 不允许的请求，状态码为403
    pub fn forbidden(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, msg)
    }

    /// 数据不存在，状态码为404
    pub fn not_found() -> Self {
        Self::new(StatusCode::NOT_FOUND, "")
    }

    /// 调用外部接口失败，状态码为502
    pub fn bad_gateway(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_GATEWAY, msg)
    }

    fn new(status: StatusCode, msg: impl Into<String>) -> Self {
        Self {
            status,
            body: msg.into(),
        }
    }
}

/// 文件操作失败时直接用 ? 返回
impl From<std::io::Error> for BizError {
    fn from(err: std::io::Error) -> Self {
        BizError::msg(format!("文件操作失败: {}", err))
    }
}

impl std::fmt::Display for BizError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.body)
    }
}

impl IntoErrorResponse for BizError {
    fn status(&self) -> StatusCode {
        self.status
    }

    fn message(&self) -> String {
        self.body.clone()
    }

    fn headers(&self) -> Vec<(&'static str, String)> {
        vec![("content-type", "text/plain; charset=utf-8".to_string())]
    }
}
