use clap::Parser;
use std::path::PathBuf;
use std::sync::LazyLock;
use tokio::sync::Notify;

/// 版本号
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 用来接收关闭通知的全局异步通知器
pub static SHUTDOWN_NOTIFY: Notify = Notify::const_new();

/// 程序启动参数
pub static ARGS: LazyLock<Argument> = LazyLock::new(Argument::parse);

#[derive(Parser, Debug)]
#[command(name = "DairoMUSIC-Search", version, about = "DairoMUSIC 歌曲搜索、采集服务")]
pub struct Argument {
    /// 监听地址(只允许本机访问时用 127.0.0.1)
    #[arg(long, env = "DAIRO_MUSIC_SEARCH_HOST", default_value = "0.0.0.0")]
    pub host: String,

    /// WEB端口
    #[arg(long, env = "DAIRO_MUSIC_SEARCH_WEB_PORT", default_value = "8021")]
    pub web_port: u16,

    /// 缓存目录(采集到的mp3保存在 <缓存目录>/music 下)
    #[arg(long, env = "DAIRO_MUSIC_SEARCH_CACHE_FOLDER", default_value = "./temp")]
    pub cache_folder: String,

    /// 数据目录(YouTube cookie 文件 youtube-cookie.txt 保存在这里)
    #[arg(long, env = "DAIRO_MUSIC_SEARCH_DATA_DIR", default_value = "./data")]
    pub data_dir: String,

    /// 允许同时采集的数量
    #[arg(long, env = "DAIRO_MUSIC_SEARCH_MAX_DOWNLOAD_THREAD_COUNT", default_value = "1",
          value_parser = clap::value_parser!(u32).range(1..))]
    pub max_download_thread_count: u32,

    /// yt-dlp 可执行文件(不在PATH里时指定完整路径)
    #[arg(long, env = "DAIRO_MUSIC_SEARCH_YT_DLP", default_value = "yt-dlp")]
    pub yt_dlp: String,
}

impl Argument {
    /// YouTube cookie 文件(由 /set/cookie 上传)
    pub fn cookie_file(&self) -> PathBuf {
        PathBuf::from(&self.data_dir).join("youtube-cookie.txt")
    }
}
