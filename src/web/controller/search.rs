use crate::service::search_service;
use crate::web::biz_error::BizError;
use hyper::HeaderMap;
use hyperoute_macro::GET;
use serde::Serialize;

/// 搜索结果
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchItem {
    /// 歌名
    pub name: String,

    /// 封面地址(通过本服务的 /d/proxy 代理下载)
    pub logo: String,

    /// 视频ID
    pub video_id: String,
}

/// 音乐搜索API
/// - key 搜索关键字,为空时返回空数组
#[GET("/search/api")]
pub async fn api(key: Option<String>, headers: &HeaderMap) -> Result<Vec<SearchItem>, BizError> {
    let key = key.unwrap_or_default();
    if key.trim().is_empty() {
        return Ok(Vec::new());
    }
    let videos = search_service::search(&key).await.map_err(|e| {
        eprintln!("-->搜索失败:{} {}", key, e);
        BizError::msg(format!("搜索失败: {}", e))
    })?;
    let origin = origin(headers);
    let list = videos
        .into_iter()
        .map(|video| {
            let logo: String = url::form_urlencoded::byte_serialize(video.logo.as_bytes()).collect();
            SearchItem {
                name: video.title,

                //通过代理的方式下载图片
                logo: format!("{}/d/proxy?url={}", origin, logo),
                video_id: video.id,
            }
        })
        .collect();
    Ok(list)
}

/// 当前访问的域名,如 http://127.0.0.1:8021
/// 经过反向代理(https)时,使用代理传过来的 X-Forwarded-Proto
fn origin(headers: &HeaderMap) -> String {
    let header = |name: &str| headers.get(name).and_then(|v| v.to_str().ok()).unwrap_or_default();
    let scheme = match header("x-forwarded-proto").split(',').next().unwrap_or_default().trim() {
        "https" => "https",
        _ => "http",
    };
    format!("{}://{}", scheme, header("host"))
}
