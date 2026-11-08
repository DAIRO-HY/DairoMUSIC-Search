//! 用 yt-dlp 采集 YouTube 视频
//! - raw: 最高规格(按分辨率从高到低,VP9/AV1 都可以),合并成mp4,给 DairoMUSIC 转码入库
//! - preview: iPhone 能直接播放的版本(最高的 H.264 + m4a),给 App 预览播放;raw 本身是 H.264/HEVC 时直接复用 raw
//!
//! 同一时间只采集一个视频(和音频的采集并发分开),其他的排队。
//! yt-dlp 在单独的工作目录里下载、合并,完成后才移动到视频目录,所以视频目录里的文件都是完整的。

use crate::application::ARGS;
use crate::service::cache_service;
use crate::service::youtube_service::{error_summary, file_size, now_millis, run, video_url, yt_dlp_command};
use crate::util::number_util;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;
use tokio::sync::Semaphore;

/// raw 的格式: 最高规格的视频 + 最好的音频,没有分开的格式时用最好的单个文件
const RAW_FORMAT: &str = "bv*+ba/b";

/// preview 的格式: 最高的 H.264 视频 + m4a(AAC) 音频
const PREVIEW_FORMAT: &str = "bv*[vcodec^=avc1]+ba[ext=m4a]/b[vcodec^=avc1]";

/// 排序: 分辨率、帧率从高到低
const FORMAT_SORT: &str = "res,fps";

/// 合并、转封装时把 moov 放到文件开头,App 才能边下边播
const FASTSTART_ARGS: [&str; 2] = ["Merger+ffmpeg_o:-movflags +faststart", "VideoRemuxer+ffmpeg_o:-movflags +faststart"];

/// 采集失败后,等待多久再重新采集
const RETRY_INTERVAL_MILLIS: i64 = 60 * 1000;

/// 单个视频采集的最长时间(4K 的长视频比较大)
const COLLECT_TIMEOUT: Duration = Duration::from_secs(2 * 60 * 60);

/// 获取视频编码的最长时间
const PROBE_TIMEOUT: Duration = Duration::from_secs(2 * 60);

/// 同一时间只采集一个视频(tokio 的 Semaphore 按先来先得的顺序分配)
static VIDEO_SEMAPHORE: Semaphore = Semaphore::const_new(1);

/// 视频类型
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum VideoType {
    Raw,
    Preview,
}

impl VideoType {
    /// 接口参数转换,不传时默认 raw
    pub fn parse(value: Option<&str>) -> Option<Self> {
        match value.unwrap_or_default() {
            "" | "raw" => Some(Self::Raw),
            "preview" => Some(Self::Preview),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::Preview => "preview",
        }
    }

    fn format(self) -> &'static str {
        match self {
            Self::Raw => RAW_FORMAT,
            Self::Preview => PREVIEW_FORMAT,
        }
    }
}

type JobKey = (String, VideoType);

/// 采集任务(排队中或采集中)
#[derive(Clone)]
struct Job {
    /// 请求采集的时间(毫秒)
    request_time: i64,

    /// 开始采集的时间(毫秒),排队中为None
    start_time: Option<i64>,

    /// 正在下载的工作目录,还没开始下载时为None
    work_dir: Option<PathBuf>,
}

/// 采集任务
static JOBS: LazyLock<Mutex<HashMap<JobKey, Job>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// 采集失败的任务 -> (失败时间(毫秒), 错误信息)
static FAILED: LazyLock<Mutex<HashMap<JobKey, (i64, String)>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// 视频保存目录
pub fn video_dir() -> PathBuf {
    PathBuf::from(&ARGS.cache_folder).join("video")
}

/// 下载中的工作目录
pub fn work_root() -> PathBuf {
    video_dir().join("work")
}

/// 采集完成的视频文件
pub fn video_file(video_id: &str, video_type: VideoType) -> PathBuf {
    video_dir().join(format!("{}.{}.mp4", video_id, video_type.as_str()))
}

fn work_dir(video_id: &str, video_type: VideoType) -> PathBuf {
    work_root().join(format!("{}.{}", video_id, video_type.as_str()))
}

/// 是否正在采集或排队(清理缓存时跳过)
pub fn is_collecting(video_id: &str) -> bool {
    JOBS.lock().unwrap().keys().any(|(id, _)| id == video_id)
}

fn update_job(key: &JobKey, f: impl FnOnce(&mut Job)) {
    if let Some(job) = JOBS.lock().unwrap().get_mut(key) {
        f(job);
    }
}

/// 采集结束时(包括出错、超时)从任务列表里移除
struct JobGuard(JobKey);

impl Drop for JobGuard {
    fn drop(&mut self) {
        JOBS.lock().unwrap().remove(&self.0);
    }
}

/// 获取视频采集情况,没有采集过时发起采集
/// 采集完成时返回 "OK",其他情况返回进度文字
pub fn video_info(video_id: &str, video_type: VideoType) -> String {
    let key: JobKey = (video_id.to_string(), video_type);
    let job = JOBS.lock().unwrap().get(&key).cloned();
    let Some(job) = job else {
        if video_file(video_id, video_type).is_file() {
            return "OK".to_string();
        }

        //刚刚采集失败时,等一段时间再重新采集
        {
            let mut failed = FAILED.lock().unwrap();
            if let Some((fail_time, msg)) = failed.get(&key) {
                let wait = RETRY_INTERVAL_MILLIS - (now_millis() - fail_time);
                if wait > 0 {
                    return format!("采集失败({}秒后重试):{}", wait / 1000 + 1, msg);
                }
                failed.remove(&key);
            }
        }
        return request_collect(key);
    };
    progress(&job)
}

/// 进度文字
fn progress(job: &Job) -> String {
    let Some(start_time) = job.start_time else {
        let timer = number_util::to_time_format((now_millis() - job.request_time) / 1000);
        let ahead = JOBS
            .lock()
            .unwrap()
            .values()
            .filter(|other| other.start_time.is_some() || other.request_time < job.request_time)
            .count();
        return format!("排队中:{},前面还有{}个视频", timer, ahead);
    };
    let timer = number_util::to_time_format((now_millis() - start_time) / 1000);
    let Some(work_dir) = &job.work_dir else {
        return format!("准备采集:{}", timer);
    };

    //yt-dlp 合并时输出到 xxx.temp.mp4
    let mut merging: Option<u64> = None;
    let mut downloaded = 0;
    if let Ok(entries) = std::fs::read_dir(work_dir) {
        for entry in entries.flatten() {
            let size = file_size(&entry.path());
            if entry.file_name().to_string_lossy().contains(".temp.") {
                merging = Some(size);
            }
            downloaded += size;
        }
    }
    match merging {
        Some(size) => format!("正在合并:{}({})", number_util::to_data_size(size), timer),
        None => format!("正在采集:{}({})", number_util::to_data_size(downloaded), timer),
    }
}

/// 发起采集(后台执行),同一时间只采集一个视频,其他的排队
fn request_collect(key: JobKey) -> String {
    let job = Job {
        request_time: now_millis(),
        start_time: None,
        work_dir: None,
    };
    let ahead = {
        let mut jobs = JOBS.lock().unwrap();
        let ahead = jobs.len();
        jobs.insert(key.clone(), job);
        ahead
    };
    let guard = JobGuard(key);
    tokio::spawn(async move {
        let (video_id, video_type) = guard.0.clone();
        let Ok(_permit) = VIDEO_SEMAPHORE.acquire().await else {
            return;
        };
        update_job(&guard.0, |job| job.start_time = Some(now_millis()));
        println!("-->开始采集视频:{} {}", video_id, video_type.as_str());
        match collect(&guard.0).await {
            Ok(()) => println!("-->视频采集完成:{} {}", video_id, video_type.as_str()),
            Err(e) => {
                eprintln!("-->视频采集失败:{} {} {}", video_id, video_type.as_str(), e);
                let mut failed = FAILED.lock().unwrap();

                //只保留最近的失败记录,避免一直增长
                failed.retain(|_, (time, _)| now_millis() - *time < RETRY_INTERVAL_MILLIS);
                failed.insert(guard.0.clone(), (now_millis(), error_summary(&e)));
            }
        }
        drop(guard);
    });
    if ahead == 0 {
        "准备采集:0S".to_string()
    } else {
        format!("排队中:0S,前面还有{}个视频", ahead)
    }
}

/// 采集视频
async fn collect(key: &JobKey) -> Result<(), String> {
    let (video_id, video_type) = (key.0.as_str(), key.1);

    //排队期间可能已经被其他任务采集好了(如 preview 复用了 raw)
    if video_file(video_id, video_type).is_file() {
        return Ok(());
    }
    match video_type {
        VideoType::Raw => collect_raw(key).await,
        VideoType::Preview => {
            //raw 已经存在却没有 preview,说明 raw 不是 iPhone 能直接播放的编码,单独下载 preview
            //raw 还不存在时,先看 raw 的编码,是 H.264/HEVC 的话直接下载 raw 当作 preview
            if !video_file(video_id, VideoType::Raw).is_file() {
                let vcodec = probe_vcodec(video_id).await?;
                if is_iphone_codec(&vcodec) {
                    println!("-->{}的最高规格是{},preview复用raw", video_id, vcodec);
                    return collect_raw(key).await;
                }
            }
            download(key, VideoType::Preview).await.map(|_| ())
        }
    }
}

/// 下载 raw,是 H.264/HEVC 时同时当作 preview
async fn collect_raw(key: &JobKey) -> Result<(), String> {
    let video_id = key.0.as_str();
    let vcodec = download(key, VideoType::Raw).await?;
    let preview = video_file(video_id, VideoType::Preview);
    if is_iphone_codec(&vcodec) && !preview.exists() {
        let raw = video_file(video_id, VideoType::Raw);

        //硬链接不占用额外空间,不支持时复制
        if std::fs::hard_link(&raw, &preview).is_err() {
            let tmp = preview.with_extension("mp4.tmp");
            std::fs::copy(&raw, &tmp).and_then(|_| std::fs::rename(&tmp, &preview)).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// iPhone 能直接播放的视频编码
fn is_iphone_codec(vcodec: &str) -> bool {
    ["avc1", "avc3", "h264", "hvc1", "hev1", "h265", "hevc"].iter().any(|c| vcodec.starts_with(c))
}

/// 获取 raw 格式的视频编码(不下载)
async fn probe_vcodec(video_id: &str) -> Result<String, String> {
    let mut cmd = yt_dlp_command();
    cmd.args(["--no-check-certificate", "--no-playlist", "-f", RAW_FORMAT, "-S", FORMAT_SORT])
        .args(["--print", "%(vcodec)s"])
        .arg(video_url(video_id))
        .stdout(Stdio::piped());
    let stdout = run(cmd, PROBE_TIMEOUT).await?;
    Ok(last_line(&stdout))
}

fn last_line(text: &str) -> String {
    text.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or_default().trim().to_string()
}

/// 下载视频到工作目录,完成后移动到视频目录,返回视频编码
async fn download(key: &JobKey, video_type: VideoType) -> Result<String, String> {
    let video_id = key.0.as_str();
    let work_dir = work_dir(video_id, video_type);
    std::fs::create_dir_all(&work_dir).map_err(|e| format!("创建目录{}失败:{}", work_dir.display(), e))?;
    update_job(key, |job| job.work_dir = Some(work_dir.clone()));

    let mut cmd = yt_dlp_command();
    cmd.args(["--no-check-certificate", "--no-playlist", "--no-mtime"])
        .args(["-f", video_type.format(), "-S", FORMAT_SORT])
        .args(["--merge-output-format", "mp4", "--remux-video", "mp4"]);
    for args in FASTSTART_ARGS {
        cmd.args(["--postprocessor-args", args]);
    }

    //全部处理完成后输出视频编码
    cmd.args(["--print", "after_move:%(vcodec)s", "-o"])
        .arg(work_dir.join("video.%(ext)s"))
        .arg(video_url(video_id))
        .stdout(Stdio::piped());
    let stdout = run(cmd, COLLECT_TIMEOUT).await?;
    let output = work_dir.join("video.mp4");
    if !output.is_file() {
        return Err(format!("没有找到合并后的文件:{}", output.display()));
    }
    let target = video_file(video_id, video_type);
    std::fs::rename(&output, &target).map_err(|e| format!("移动文件失败:{}", e))?;
    cache_service::touch(&target);
    let _ = tokio::fs::remove_dir_all(&work_dir).await;
    let vcodec = last_line(&stdout);
    println!(
        "-->视频下载完成:{} {} {} {}",
        video_id,
        video_type.as_str(),
        vcodec,
        number_util::to_data_size(file_size(&target))
    );
    Ok(vcodec)
}

/// 视频文件对应的视频ID(文件名 "{videoId}.{type}.mp4" 或工作目录 "{videoId}.{type}")
pub fn video_id_of(name: &str) -> &str {
    name.split('.').next().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn video_type() {
        assert_eq!(VideoType::parse(None), Some(VideoType::Raw));
        assert_eq!(VideoType::parse(Some("")), Some(VideoType::Raw));
        assert_eq!(VideoType::parse(Some("raw")), Some(VideoType::Raw));
        assert_eq!(VideoType::parse(Some("preview")), Some(VideoType::Preview));
        assert_eq!(VideoType::parse(Some("4k")), None);
    }

    #[test]
    fn iphone_codec() {
        assert!(is_iphone_codec("avc1.640028"));
        assert!(is_iphone_codec("hev1.1.6.L120.90"));
        assert!(!is_iphone_codec("vp09.00.51.08"));
        assert!(!is_iphone_codec("av01.0.12M.08"));
    }
}
