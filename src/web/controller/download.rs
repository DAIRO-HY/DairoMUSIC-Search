use crate::service::video_service::{self, VideoType};
use crate::service::{cache_service, youtube_service};
use crate::web::form::VideoForm;
use crate::web::biz_error::BizError;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{HeaderMap, StatusCode};
use hyperoute_core::{Query, Raw, Stream};
use hyperoute_macro::GET;
use std::sync::LazyLock;
use std::time::Duration;

/// 代理下载的图片最大大小
const PROXY_MAX_SIZE: usize = 10 * 1024 * 1024;

/// 代理下载图片的客户端,重定向的目标也必须是允许的域名
static PROXY_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    let redirect = reqwest::redirect::Policy::custom(|attempt| {
        if attempt.previous().len() >= 5 || !is_allowed_proxy_url(attempt.url()) {
            attempt.stop()
        } else {
            attempt.follow()
        }
    });
    reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .redirect(redirect)
        .build()
        .expect("创建HTTP客户端失败")
});

/// 检查视频ID,防止拼接路径时访问到其他目录
fn check_video_id(video_id: &str) -> Result<(), BizError> {
    if youtube_service::is_valid_video_id(video_id) {
        Ok(())
    } else {
        Err(BizError::param("videoId格式错误"))
    }
}

/// 音乐采集情况,没有采集过时发起采集
/// 采集完成时返回 "OK",其他情况返回进度文字,如 "正在采集:1.20MB(00:10)"
/// - video_id 视频ID
#[GET("/d/collect_info")]
pub async fn collect_info(video_id: String) -> Result<String, BizError> {
    check_video_id(&video_id)?;
    Ok(youtube_service::collect_info(&video_id))
}

/// 下载采集好的mp3,支持Range
/// 正在转码时也可以下载(边转码边播放),文件不存在时返回404
/// - video_id 视频ID
#[GET("/d/music")]
pub async fn music(video_id: String, headers: &HeaderMap) -> Result<Stream, BizError> {
    //兼容旧版本: /d/music?videoId=xxx/collect_info 是获取采集情况
    if let Some(video_id) = video_id.strip_suffix("/collect_info") {
        check_video_id(video_id)?;
        return Ok(text_stream(youtube_service::collect_info(video_id)));
    }
    check_video_id(&video_id)?;
    let file = youtube_service::music_file(&video_id);
    let stream = Stream::file(&file, headers).await.map_err(|_| BizError::not_found())?;
    cache_service::touch(&file);
    Ok(stream.content_type("audio/mpeg"))
}

/// 检查视频采集、下载的参数
fn video_param(form: &VideoForm) -> Result<(&str, VideoType), BizError> {
    check_video_id(&form.video_id)?;
    let video_type =
        VideoType::parse(form.video_type.as_deref()).ok_or_else(|| BizError::param("type只能是raw或preview"))?;
    Ok((&form.video_id, video_type))
}

/// 视频采集情况,没有采集过时发起采集
/// 采集完成时返回 "OK",其他情况返回进度文字,如 "正在采集:12.30MB(00:10)"
/// - video_id 视频ID
/// - type raw(最高规格,默认) / preview(iPhone能直接播放的版本)
#[GET("/d/video_info")]
pub async fn video_info(form: Query<VideoForm>) -> Result<String, BizError> {
    let (video_id, video_type) = video_param(&form)?;
    Ok(video_service::video_info(video_id, video_type))
}

/// 下载采集好的视频(mp4),支持Range
/// 没有采集完成时返回404
/// - video_id 视频ID
/// - type raw(最高规格,默认) / preview(iPhone能直接播放的版本)
#[GET("/d/video")]
pub async fn video(form: Query<VideoForm>, headers: &HeaderMap) -> Result<Stream, BizError> {
    let (video_id, video_type) = video_param(&form)?;
    let file = video_service::video_file(video_id, video_type);
    let stream = Stream::file(&file, headers).await.map_err(|_| BizError::not_found())?;
    cache_service::touch(&file);
    Ok(stream.content_type("video/mp4"))
}

/// 纯文本响应
fn text_stream(text: String) -> Stream {
    Stream {
        status: StatusCode::OK,
        headers: vec![("content-type", "text/plain; charset=utf-8".to_string())],
        body: hyperoute_core::boxed_full(Full::new(Bytes::from(text))),
    }
}

/// 下载歌词,没有歌词时返回空字符串
/// - video_id 视频ID
#[GET("/d/lrc")]
pub async fn lrc(video_id: String) -> Result<String, BizError> {
    check_video_id(&video_id)?;
    Ok(youtube_service::get_lrc(&video_id).await)
}

/// 代理下载封面图片(App无法直接访问YouTube时使用)
/// 只允许YouTube的图片域名,避免被当成开放代理
/// - url 图片地址
#[GET("/d/proxy")]
pub async fn proxy(url: String) -> Result<Raw, BizError> {
    let target = url::Url::parse(&url).map_err(|_| BizError::param("url格式错误"))?;
    if !is_allowed_proxy_url(&target) {
        return Err(BizError::forbidden("只允许代理YouTube的图片"));
    }
    let mut resp = PROXY_CLIENT.get(target).send().await.map_err(proxy_error)?;
    if resp.content_length().is_some_and(|len| len > PROXY_MAX_SIZE as u64) {
        return Err(BizError::bad_gateway("图片太大"));
    }
    let status = resp.status();
    let content_type = resp
        .headers()
        .get(hyper::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();
    let mut body = Vec::new();
    while let Some(chunk) = resp.chunk().await.map_err(proxy_error)? {
        if body.len() + chunk.len() > PROXY_MAX_SIZE {
            return Err(BizError::bad_gateway("图片太大"));
        }
        body.extend_from_slice(&chunk);
    }
    let mut raw = Raw::new(body)
        .status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY))
        .header("content-type", content_type);
    if status.is_success() {
        raw = raw.header("cache-control", "public, max-age=86400");
    }
    Ok(raw)
}

fn proxy_error(e: reqwest::Error) -> BizError {
    BizError::bad_gateway(format!("下载图片失败: {}", e.without_url()))
}

/// 只允许 ytimg.com 及其子域名(i.ytimg.com、i9.ytimg.com 等)
fn is_allowed_proxy_url(url: &url::Url) -> bool {
    if !matches!(url.scheme(), "http" | "https") {
        return false;
    }
    match url.host() {
        Some(url::Host::Domain(host)) => {
            let host = host.trim_end_matches('.').to_ascii_lowercase();
            host == "ytimg.com" || host.ends_with(".ytimg.com")
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allowed(url: &str) -> bool {
        is_allowed_proxy_url(&url::Url::parse(url).unwrap())
    }

    #[test]
    fn proxy_whitelist() {
        assert!(allowed("https://i.ytimg.com/vi/abc/hqdefault.jpg"));
        assert!(allowed("https://i9.ytimg.com/vi/abc/hqdefault.jpg"));
        assert!(allowed("http://I.YTIMG.COM./vi/abc/hqdefault.jpg"));
        assert!(!allowed("https://evilytimg.com/a.jpg"));
        assert!(!allowed("https://i.ytimg.com.evil.com/a.jpg"));
        assert!(!allowed("https://127.0.0.1/a.jpg"));
        assert!(!allowed("http://localhost:8021/a.jpg"));
        assert!(!allowed("file:///etc/passwd"));
    }
}
