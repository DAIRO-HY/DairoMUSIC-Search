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
    service::cache_service::start();
    web::server::ready();

    //收到Ctrl+C时退出
    tokio::spawn(async {
        if tokio::signal::ctrl_c().await.is_ok() {
            application::SHUTDOWN_NOTIFY.notify_waiters();
        }
    });

    //等待程序退出
    application::SHUTDOWN_NOTIFY.notified().await;
    println!("-->即将退出");

    // 等待一段时间再退出确保监听端口都已经关闭
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    println!("-->程序已退出");
}
