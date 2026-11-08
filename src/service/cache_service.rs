//! 缓存清理: 每天清理一次,删除24小时没有被访问过的 mp3、视频、字幕临时目录
//! "访问"指 /d/music、/d/video 被请求,或者被采集,访问时把文件的访问时间(atime)改成当前时间。
//! 用 atime 而不是修改时间,是因为下载接口的 ETag 按修改时间生成,改了会导致客户端断点续传失败。

use crate::service::{video_service, youtube_service};
use std::fs::{File, FileTimes};
use std::path::Path;
use std::time::{Duration, SystemTime};

/// 多久没有被访问过的文件会被删除
const MAX_IDLE: Duration = Duration::from_secs(24 * 60 * 60);

/// 清理间隔
const CLEAN_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// 记录文件被访问
pub fn touch(path: &Path) {
    let result = File::open(path).and_then(|f| f.set_times(FileTimes::new().set_accessed(SystemTime::now())));
    if let Err(e) = result {
        eprintln!("-->更新访问时间失败:{} {}", path.display(), e);
    }
}

/// 启动定时清理(启动时先清理一次)
pub fn start() {
    tokio::spawn(async {
        let mut interval = tokio::time::interval(CLEAN_INTERVAL);
        loop {
            interval.tick().await;
            let _ = tokio::task::spawn_blocking(clean).await;
        }
    });
}

fn clean() {
    let now = SystemTime::now();
    let mut count = 0;

    //mp3 和下载中的临时文件,文件名以视频ID开头
    count += clean_dir(&youtube_service::music_dir(), now, |name| {
        youtube_service::is_collecting(name.split('.').next().unwrap_or_default())
    });

    //视频(工作目录 work 单独处理)
    count += clean_dir(&video_service::video_dir(), now, |name| {
        name == "work" || video_service::is_collecting(video_service::video_id_of(name))
    });

    //视频下载中断留下的工作目录
    count += clean_dir(&video_service::work_root(), now, |name| {
        video_service::is_collecting(video_service::video_id_of(name))
    });

    //获取字幕时的临时目录(正常情况下获取完就删除了)
    count += clean_dir(&youtube_service::lrc_dir(), now, |_| false);
    println!("-->缓存清理完成,删除了{}个文件", count);
}

/// 清理目录下长时间没有访问的文件(或子目录),skip 返回 true 的跳过
fn clean_dir(dir: &Path, now: SystemTime, skip: impl Fn(&str) -> bool) -> usize {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut count = 0;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if skip(&name) {
            continue;
        }
        let path = entry.path();
        let idle = now.duration_since(last_access(&path)).unwrap_or_default();
        if idle < MAX_IDLE {
            continue;
        }
        let result = if path.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
        match result {
            Ok(()) => {
                println!("-->删除缓存:{}", path.display());
                count += 1;
            }
            Err(e) => eprintln!("-->删除缓存失败:{} {}", path.display(), e),
        }
    }
    count
}

/// 最后访问时间: 文件取访问时间和修改时间较新的一个,目录取里面所有文件中最新的
fn last_access(path: &Path) -> SystemTime {
    let Ok(meta) = std::fs::metadata(path) else {
        return SystemTime::UNIX_EPOCH;
    };
    let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    if !meta.is_dir() {
        return meta.accessed().map_or(modified, |accessed| accessed.max(modified));
    }

    //目录的访问时间在列出文件时就会变,只看修改时间
    let mut time = modified;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            time = time.max(last_access(&entry.path()));
        }
    }
    time
}
