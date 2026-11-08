use serde::Deserialize;

/// 上传YouTube cookie的表单
/// 放在这里而不是和handler同一个文件:同文件的结构体会生成multipart解析代码
#[derive(Deserialize, Debug)]
pub struct CookieForm {
    /// cookie文件内容(Netscape格式,用 Get cookies.txt 插件从Chrome导出)
    #[serde(default)]
    pub cookie: String,
}

/// 视频采集、下载的参数
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VideoForm {
    /// 视频ID
    #[serde(default)]
    pub video_id: String,

    /// 视频类型: raw(最高规格,默认) / preview(iPhone能直接播放的版本)
    #[serde(rename = "type", default)]
    pub video_type: Option<String>,
}
