mod application;
mod service;
mod util;
mod web;

#[tokio::main]
async fn main() {
    println!("-->DairoMUSIC-Search {} 启动中", application::VERSION);
    let args = &*application::ARGS;

    //缓存目录、数据目录不存在时创建
    for dir in [service::youtube_service::music_dir(), service::video_service::work_root(), args.data_dir.clone().into()] {
        if let Err(e) = std::fs::create_dir_all(&dir) {
            eprintln!("-->创建目录:{}失败: {}", dir.display(), e);
            std::process::exit(1);
        }
    }
    service::youtube_service::clean_tmp_files();
    service::cache_service::start();
    web::server::ready();

    //收到Ctrl+C(SIGINT)、SIGTERM时退出
    tokio::spawn(async {
        wait_exit_signal().await;
        application::SHUTDOWN_NOTIFY.notify_waiters();
    });

    //等待程序退出
    application::SHUTDOWN_NOTIFY.notified().await;
    println!("-->即将退出");

    //结束正在运行的yt-dlp、ffmpeg,先通知结束,1秒后还没结束的强制结束
    let count = util::process_util::kill_all(util::process_util::Signal::Term);
    if count > 0 {
        println!("-->结束{}个正在运行的采集任务", count);
    }

    // 等待一段时间再退出确保监听端口都已经关闭
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    util::process_util::kill_all(util::process_util::Signal::Kill);
    println!("-->程序已退出");
}

/// 等待退出信号
async fn wait_exit_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut term = signal(SignalKind::terminate()).expect("监听SIGTERM失败");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = term.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
