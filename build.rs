use hyperoute_codegen::ParamNaming;
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let src_dir = manifest_dir.join("src");

    // 扫描controller下的注解，生成路由代码
    let routes = hyperoute_codegen::Scanner::new()
        .dir(src_dir.join("web/controller"), "crate::web::controller")
        // 普通参数同时接收驼峰（videoId）和Rust原名（video_id）
        .param_naming(ParamNaming::CamelCase)
        .scan();
    let code = hyperoute_codegen::generate_router_source(&routes);
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("routes.rs");
    fs::write(&out_path, code).unwrap();

    // 追踪输入文件变化
    println!("cargo:rerun-if-changed=src/web/controller");
}
