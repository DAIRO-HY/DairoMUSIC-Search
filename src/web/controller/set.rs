use crate::application::ARGS;
use crate::web::biz_error::BizError;
use crate::web::form::CookieForm;
use hyperoute_core::Raw;
use hyperoute_macro::{GET, POST};

/// 上传YouTube cookie的页面
#[GET("/set")]
pub async fn set_page() -> Raw {
    Raw::new(include_str!("../../../assets/resources/set.html")).header("content-type", "text/html; charset=utf-8")
}

/// 保存YouTube cookie(yt-dlp 采集时使用)
#[POST("/set/cookie")]
pub async fn cookie(form: CookieForm) -> Result<String, BizError> {
    let cookie_file = ARGS.cookie_file();

    //先写到临时文件再改名,避免yt-dlp读到写了一半的文件
    let temp_file = cookie_file.with_extension("txt.tmp");
    tokio::fs::write(&temp_file, form.cookie.as_bytes()).await?;
    tokio::fs::rename(&temp_file, &cookie_file).await?;
    Ok("上传成功".to_string())
}
