use thiserror::Error;

/// 认证与授权错误。
#[derive(Debug, Error, Clone, PartialEq)]
pub enum AuthError {
    #[error("账号或密码错误")]
    InvalidCredentials,
    #[error("会话已过期，请重新登录")]
    SessionExpired,
    #[error("无权限访问该资源")]
    Forbidden,
    #[error("未提供凭据")]
    MissingCredentials,
    #[error("其他认证错误: {0}")]
    Other(String),
}

/// 数据查询错误。
#[derive(Debug, Error, Clone, PartialEq)]
pub enum QueryError {
    #[error("资源不存在")]
    NotFound,
    #[error("无权限查看该资源")]
    Forbidden,
    #[error("会话已过期")]
    SessionExpired,
    #[error("请求参数无效: {0}")]
    InvalidParameter(String),
    #[error("远端响应无法解析: {0}")]
    ParseError(String),
    #[error("远端返回不兼容结构")]
    IncompatibleResponse,
    #[error("网络或远端错误: {0}")]
    Remote(String),
}

/// 汇总 CLI 可能遇到的领域错误。
#[derive(Debug, Error, Clone, PartialEq)]
pub enum ZentaoError {
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error(transparent)]
    Query(#[from] QueryError),
    #[error("内部错误: {0}")]
    Internal(String),
}
