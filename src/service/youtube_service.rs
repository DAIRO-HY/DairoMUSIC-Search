//! 用 yt-dlp 采集 YouTube 的音频和字幕

use crate::application::ARGS;
use crate::service::cache_service;
use crate::util::number_util;
use crate::util::process_util::{self, ChildGuard};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::process::Command;

/// 正在采集中的视频ID -> 采集开始时间(毫秒)
static COLLECTING: LazyLock<Mutex<HashMap<String, i64>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// 采集失败的视频ID -> (失败时间(毫秒), 错误信息)
static FAILED: LazyLock<Mutex<HashMap<String, (i64, String)>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// 采集失败后,等待多久再重新采集(避免每次查询都立即重试)
const RETRY_INTERVAL_MILLIS: i64 = 60 * 1000;

/// 单首歌曲采集的最长时间,超过后结束yt-dlp,避免一直占用采集名额
const COLLECT_TIMEOUT: Duration = Duration::from_secs(30 * 60);

/// 获取字幕的最长时间(DairoMUSIC 调用 /d/lrc 的超时是120秒)
const LRC_TIMEOUT: Duration = Duration::from_secs(110);

/// 多个字幕之间的分割线
const LRC_SPLIT: &str = "\n<----------->\n";

/// 歌曲保存目录
pub fn music_dir() -> PathBuf {
    PathBuf::from(&ARGS.cache_folder).join("music")
}

/// 采集完成的mp3文件
pub fn music_file(video_id: &str) -> PathBuf {
    music_dir().join(format!("{}.mp3", video_id))
}

/// 采集中的临时文件名前缀
/// yt-dlp 先下载到 <videoId>.tmp.webm(或.m4a等),再转码成 <videoId>.tmp.mp3,完成后才重命名成 <videoId>.mp3,
/// 这样采集中途被强制结束时,不会把不完整的mp3当成采集完成
fn tmp_prefix(video_id: &str) -> String {
    format!("{}.tmp.", video_id)
}

/// 删除上次没有采集完成留下的临时文件(启动时调用)
pub fn clean_tmp_files() {
    let Ok(entries) = std::fs::read_dir(music_dir()) else {
        return;
    };
    for entry in entries.flatten() {
        if entry.file_name().to_string_lossy().contains(".tmp.") {
            match std::fs::remove_file(entry.path()) {
                Ok(()) => println!("-->删除没有采集完成的文件:{}", entry.path().display()),
                Err(e) => eprintln!("-->删除没有采集完成的文件:{}失败: {}", entry.path().display(), e),
            }
        }
    }
}

/// 视频ID只允许字母、数字、- 和 _,防止拼接路径时访问到其他目录
pub fn is_valid_video_id(video_id: &str) -> bool {
    !video_id.is_empty()
        && video_id.len() <= 64
        && video_id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

pub(crate) fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}

/// 采集开始时间,不在采集中时返回None
fn collect_start_time(video_id: &str) -> Option<i64> {
    COLLECTING.lock().unwrap().get(video_id).copied()
}

/// 当前采集数
fn collecting_count() -> usize {
    COLLECTING.lock().unwrap().len()
}

/// 是否正在采集(清理缓存时跳过)
pub fn is_collecting(video_id: &str) -> bool {
    COLLECTING.lock().unwrap().contains_key(video_id)
}

/// 采集结束时(包括出错、超时)从采集列表里移除
struct CollectingGuard(String);

impl Drop for CollectingGuard {
    fn drop(&mut self) {
        COLLECTING.lock().unwrap().remove(&self.0);
    }
}

/// 获取采集情况,没有采集过时发起采集
/// 采集完成时返回 "OK",其他情况返回进度文字
pub fn collect_info(video_id: &str) -> String {
    let start_time = collect_start_time(video_id);

    //已经耗时
    let timer = match start_time {
        Some(start) => number_util::to_time_format((now_millis() - start) / 1000),
        None => "0S".to_string(),
    };

    //采集完成后才会有mp3文件
    if start_time.is_none() && music_file(video_id).is_file() {
        return "OK".to_string();
    }

    //获取采集中的临时文件
    let mut download_file: Option<PathBuf> = None;
    let mut mp3_file: Option<PathBuf> = None;
    if start_time.is_some() {
        let prefix = tmp_prefix(video_id);
        if let Ok(entries) = std::fs::read_dir(music_dir()) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if !name.starts_with(&prefix) {
                    continue;
                }
                if name.ends_with(".mp3") {
                    //正在转码的mp3
                    mp3_file = Some(entry.path());
                } else {
                    //正在下载的音频
                    download_file = Some(entry.path());
                }
            }
        }
    }
    if let Some(mp3_file) = mp3_file {
        return format!("正在转码:{}({})", number_util::to_data_size(file_size(&mp3_file)), timer);
    }
    if let Some(download_file) = download_file {
        return format!("正在采集:{}({})", number_util::to_data_size(file_size(&download_file)), timer);
    }
    if start_time.is_some() {
        return format!("准备采集:{}", timer);
    }

    //刚刚采集失败时,等一段时间再重新采集
    if start_time.is_none() {
        let mut failed = FAILED.lock().unwrap();
        if let Some((fail_time, msg)) = failed.get(video_id) {
            let wait = RETRY_INTERVAL_MILLIS - (now_millis() - fail_time);
            if wait > 0 {
                return format!("采集失败({}秒后重试):{}", wait / 1000 + 1, msg);
            }
            failed.remove(video_id);
        }
    }

    //还没有采集,去采集
    let count = collecting_count();
    request_collect(video_id);
    if count == 0 {
        format!("准备采集:{}", timer)
    } else {
        format!("准备采集{},当前采集数:{}", timer, count)
    }
}

pub(crate) fn file_size(path: &Path) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or_default()
}

/// 发起采集(后台执行)
/// 已经在采集中、或者达到最大采集数时忽略,等下次查询采集情况时再发起
fn request_collect(video_id: &str) {
    {
        let mut collecting = COLLECTING.lock().unwrap();
        if collecting.contains_key(video_id) {
            return;
        }
        if collecting.len() >= ARGS.max_download_thread_count as usize {
            println!("-->达到最大采集并发数,{}稍后再采集", video_id);
            return;
        }
        collecting.insert(video_id.to_string(), now_millis());
    }
    let guard = CollectingGuard(video_id.to_string());
    tokio::spawn(async move {
        let video_id = guard.0.clone();
        collect_music(&video_id).await;
        drop(guard);
    });
}

/// 采集音频并转码成mp3
/// yt-dlp中,-o - 以流的形式输出时,--audio-format --audio-quality这样的参数无效,输出的文件格式为opus,
/// 所以这里直接输出到文件
async fn collect_music(video_id: &str) {
    let mp3_file = music_file(video_id);
    let tmp_file = music_dir().join(format!("{}mp3", tmp_prefix(video_id)));
    let mut cmd = yt_dlp_command();
    cmd.args(["--no-check-certificate", "--no-mtime", "-x", "--audio-format", "mp3", "--audio-quality", "0", "-o"])
        .arg(music_dir().join(format!("{}%(ext)s", tmp_prefix(video_id))))
        .arg(video_url(video_id));
    println!("-->开始采集:{}", video_id);
    let result = run(cmd, COLLECT_TIMEOUT).await.and_then(|_| {
        //转码完成后才重命名成正式的文件名
        std::fs::rename(&tmp_file, &mp3_file).map_err(|e| format!("重命名{}失败: {}", tmp_file.display(), e))
    });
    match result {
        Ok(()) => {
            cache_service::touch(&mp3_file);
            println!("-->采集完成:{}", video_id)
        }
        Err(e) => {
            eprintln!("-->采集失败:{} {}", video_id, e);
            let mut failed = FAILED.lock().unwrap();

            //只保留最近的失败记录,避免一直增长
            failed.retain(|_, (time, _)| now_millis() - *time < RETRY_INTERVAL_MILLIS);
            failed.insert(video_id.to_string(), (now_millis(), error_summary(&e)));
        }
    }
}

/// yt-dlp的错误输出里有很多WARNING,只取ERROR那一行显示给用户
pub(crate) fn error_summary(stderr: &str) -> String {
    let line = stderr
        .lines()
        .rev()
        .find(|line| line.starts_with("ERROR:"))
        .or_else(|| stderr.lines().rev().find(|line| !line.trim().is_empty()))
        .unwrap_or_default();
    let line = line.strip_prefix("ERROR:").unwrap_or(line).trim();
    line.chars().take(200).collect()
}

/// 字幕临时目录
pub fn lrc_dir() -> PathBuf {
    PathBuf::from(&ARGS.cache_folder).join("lrc")
}

/// 获取字幕,没有字幕时返回空字符串
pub async fn get_lrc(video_id: &str) -> String {
    //每次请求用单独的目录,同一首歌同时请求时互不影响
    let folder = lrc_dir().join(format!("{}-{}", video_id, now_millis()));
    let mut cmd = yt_dlp_command();
    cmd.args(["--no-check-certificate", "--write-subs", "--sub-langs", "all", "--skip-download"])
        .arg(video_url(video_id))
        .arg("-o")
        .arg(folder.join("lrc"));
    if let Err(e) = run(cmd, LRC_TIMEOUT).await {
        eprintln!("-->获取字幕失败:{} {}", video_id, e);
    }

    //可能有多个语言的字幕,按文件名排序后合并
    let mut files: Vec<PathBuf> = match std::fs::read_dir(&folder) {
        Ok(entries) => entries.flatten().map(|e| e.path()).filter(|p| p.is_file()).collect(),
        Err(_) => Vec::new(),
    };
    files.sort();
    let lrc = files
        .iter()
        .filter_map(|f| std::fs::read(f).ok())
        .map(|data| String::from_utf8_lossy(&data).into_owned())
        .collect::<Vec<_>>()
        .join(LRC_SPLIT);
    let _ = tokio::fs::remove_dir_all(&folder).await;
    lrc
}

pub(crate) fn video_url(video_id: &str) -> String {
    format!("https://www.youtube.com/watch?v={}", video_id)
}

/// yt-dlp指令,cookie文件存在时带上cookie
pub(crate) fn yt_dlp_command() -> Command {
    let mut cmd = Command::new(&ARGS.yt_dlp);
    let cookie_file = ARGS.cookie_file();
    if cookie_file.is_file() {
        cmd.arg("--cookies").arg(cookie_file);
    }
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped());
    process_util::prepare(&mut cmd);
    cmd
}

/// 执行指令,成功时返回标准输出(没有设置stdout为piped时为空),失败时返回错误输出
pub(crate) async fn run(mut cmd: Command, timeout: Duration) -> Result<String, String> {
    let child = cmd.spawn().map_err(|e| format!("执行{}失败: {}", ARGS.yt_dlp, e))?;

    //超时、请求被取消、程序退出时,guard 结束 yt-dlp 和它启动的 ffmpeg
    let guard = ChildGuard::new(&child);
    let result = tokio::time::timeout(timeout, child.wait_with_output()).await;
    if result.is_ok() {
        guard.finish();
    }
    match result {
        Err(_) => Err(format!("超过{}秒没有完成", timeout.as_secs())),
        Ok(Err(e)) => Err(e.to_string()),
        Ok(Ok(output)) if output.status.success() => Ok(String::from_utf8_lossy(&output.stdout).into_owned()),
        Ok(Ok(output)) => Err(String::from_utf8_lossy(&output.stderr).trim().to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn video_id() {
        assert!(is_valid_video_id("Ag1ijDkufy8"));
        assert!(is_valid_video_id("a-b_c"));
        assert!(!is_valid_video_id(""));
        assert!(!is_valid_video_id("../etc/passwd"));
        assert!(!is_valid_video_id("abc.mp3"));
    }
}
