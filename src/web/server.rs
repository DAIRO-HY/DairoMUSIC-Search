use crate::application;
use tokio::net::TcpListener;
use tokio::select;

/// 由build.rs扫描注解生成的路由
mod routes {
    include!(concat!(env!("OUT_DIR"), "/routes.rs"));
}

pub fn ready() {
    tokio::spawn(start());
}

async fn start() {
    let args = &*application::ARGS;
    let listener = TcpListener::bind((args.host.as_str(), args.web_port)).await.unwrap_or_else(|e| {
        eprintln!("-->Web服务监听{}:{}失败: {}", args.host, args.web_port, e);
        std::process::exit(1);
    });
    println!("-->Web服务已启动,地址:{}:{}", args.host, args.web_port);
    loop {
        select! {
            _ = application::SHUTDOWN_NOTIFY.notified() => break,
            accepted = listener.accept() => match accepted {
                Ok((stream, peer)) => {
                    //带上客户端地址,handler里可以通过RemoteAddr获取客户端IP
                    tokio::spawn(hyperoute_core::serve_connection_with_addr(stream, peer, routes::router));
                }
                Err(e) => eprintln!("Web服务接收连接失败:{:?}", e),
            }
        }
    }
}
