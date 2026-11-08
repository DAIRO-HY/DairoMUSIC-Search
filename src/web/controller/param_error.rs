use crate::web::biz_error::BizError;
use hyperoute_core::{ParamErrorKind, ParamErrors};
use hyperoute_macro::PARAM_ERROR;

/// 请求参数解析失败时，返回纯文本的错误提示
#[PARAM_ERROR]
pub fn on_param_error(errors: ParamErrors) -> BizError {
    let e = errors.first();
    let Some(param) = &e.param else {
        //针对整个请求的错误，比如请求体格式错误
        return BizError::param(format!("请求数据格式错误: {}", e));
    };
    let msg = match e.kind {
        ParamErrorKind::Missing => "该字段必须",
        ParamErrorKind::OutOfRange => "数值超出范围",
        _ => "类型转换失败",
    };
    BizError::param(format!("{}:{}", param, msg))
}
