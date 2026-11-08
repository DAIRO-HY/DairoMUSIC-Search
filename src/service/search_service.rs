//! 用 yt-dlp 搜索 YouTube 视频(ytsearch),不需要谷歌开发者KEY

use crate::service::youtube_service::{error_summary, is_valid_video_id, run, yt_dlp_command};
use serde_json::Value;
use std::process::Stdio;
use std::time::Duration;

/// 最大检索结果数
const LIMIT: u32 = 30;

/// 搜索的最长时间(DairoMUSIC 调用搜索接口的超时是10秒)
const SEARCH_TIMEOUT: Duration = Duration::from_secs(9);

/// 搜索到的视频
pub struct Video {
    /// 视频ID
    pub id: String,

    /// 标题
    pub title: String,

    /// 封面地址
    pub logo: String,
}

/// 搜索视频
pub async fn search(key: &str) -> Result<Vec<Video>, String> {
    let mut cmd = yt_dlp_command();

    //--flat-playlist: 只获取搜索结果列表,不解析每个视频(快很多)
    cmd.args(["--no-check-certificate", "--flat-playlist", "-J"])
        .arg(format!("ytsearch{}:{}", LIMIT, key))
        .stdout(Stdio::piped());
    let stdout = run(cmd, SEARCH_TIMEOUT).await.map_err(|e| error_summary(&e))?;
    let data: Value = serde_json::from_str(&stdout).map_err(|e| format!("解析搜索结果失败: {}", e))?;
    Ok(parse(&data))
}

/// 解析 yt-dlp 输出的搜索结果
fn parse(data: &Value) -> Vec<Video> {
    let entries = data.get("entries").and_then(Value::as_array).map(Vec::as_slice).unwrap_or_default();
    entries
        .iter()
        .filter_map(|entry| {
            //只要视频(搜索结果里也可能有频道、播放列表)
            if entry["ie_key"].as_str().is_some_and(|ie| ie != "Youtube") {
                return None;
            }
            let id = entry["id"].as_str().filter(|id| is_valid_video_id(id))?;
            Some(Video {
                id: id.to_string(),
                title: entry["title"].as_str().unwrap_or_default().to_string(),

                //和之前 YouTube Data API 返回的 thumbnails.high 一样(480x360)
                logo: format!("https://i.ytimg.com/vi/{}/hqdefault.jpg", id),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_entries() {
        let data: Value = serde_json::from_str(
            r#"{"entries":[
                {"_type":"url","ie_key":"Youtube","id":"o9cCch2Rqak","title":"江濤 - 生日禮物 'Live'"},
                {"_type":"url","ie_key":"YoutubeTab","id":"UC8ipZTdJfpg7JY4if-oRifQ","title":"频道"},
                {"_type":"url","ie_key":"Youtube","id":"../x","title":"非法ID"}
            ]}"#,
        )
        .unwrap();
        let list = parse(&data);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "o9cCch2Rqak");
        assert_eq!(list[0].title, "江濤 - 生日禮物 'Live'");
        assert_eq!(list[0].logo, "https://i.ytimg.com/vi/o9cCch2Rqak/hqdefault.jpg");
        assert!(parse(&Value::Null).is_empty());
    }
}
