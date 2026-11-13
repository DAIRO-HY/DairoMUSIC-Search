# DairoMUSIC-Search

DairoMUSIC 的歌曲搜索、采集服务：用 [yt-dlp](https://github.com/yt-dlp/yt-dlp) 搜索 YouTube 歌曲（不需要谷歌开发者 KEY），采集音频（转码成 mp3）、视频和字幕。

# 1. 准备

- 安装 [yt-dlp](https://github.com/yt-dlp/yt-dlp)（请保持最新版本，旧版本经常被 YouTube 拒绝下载），以及它推荐的 JS 运行时（如 deno）
- 安装 [FFmpeg](https://ffmpeg.org/)（yt-dlp 转码 mp3 时使用）

# 2. 编译和运行

需要安装 [Rust](https://www.rust-lang.org/)（stable 版本）。

```bash
#开发时运行
cargo run

#编译发布版本
cargo build --release

#运行
./target/release/DairoMUSIC-Search
```

发布版（GitHub Release、Docker 镜像）由 `document/build/build_all.sh` 编译，和 DairoMUSIC 的方式相同：

- Linux amd64/arm64：在官方 `rust:1.99.1` 镜像（Debian 13）里编译，glibc 动态链接，**需要 glibc 2.39 以上**（Debian 13、Ubuntu 24.04 及更新的系统）。旧版本的 Linux 请用 Docker 运行（`document/docker/docker-compose.yml`）
- Windows amd64/arm64、macOS amd64/arm64：用 cargo-zigbuild 交叉编译，macOS 版需要 macOS SDK（在 Mac 上执行 `document/build/pack_macos_sdk.sh` 打包）

# 3. 启动参数

所有参数也可以用环境变量设置（`--help` 查看）。

| 参数 | 环境变量 | 默认值 | 说明 |
|---|---|---|---|
| `--host` | `DAIRO_MUSIC_SEARCH_HOST` | 0.0.0.0 | 监听地址，只允许本机访问时用 127.0.0.1 |
| `--web-port` | `DAIRO_MUSIC_SEARCH_WEB_PORT` | 8021 | WEB 端口 |
| `--cache-folder` | `DAIRO_MUSIC_SEARCH_CACHE_FOLDER` | ./temp | 缓存目录，mp3 保存在 `<缓存目录>/music` |
| `--data-dir` | `DAIRO_MUSIC_SEARCH_DATA_DIR` | ./data | 数据目录，YouTube cookie 保存在 `<数据目录>/youtube-cookie.txt` |
| `--max-download-thread-count` | `DAIRO_MUSIC_SEARCH_MAX_DOWNLOAD_THREAD_COUNT` | 1 | 允许同时采集的数量 |
| `--yt-dlp` | `DAIRO_MUSIC_SEARCH_YT_DLP` | yt-dlp | yt-dlp 可执行文件 |

cookie 文件是敏感信息，不要提交到代码仓库（`data/`、`temp/` 已经在 .gitignore 里）。

# 4. 视频采集

- raw：最高规格，`-f "bv*+ba/b" -S res,fps`，合并成 mp4（VP9/AV1 + Opus 都有可能）
- preview：iPhone 能直接播放的版本，`-f "bv*[vcodec^=avc1]+ba[ext=m4a]/b[vcodec^=avc1]" -S res,fps`（H.264 + AAC，一般 1080p）。raw 本身是 H.264/HEVC 时直接复用 raw（硬链接）
- 都加了 faststart（moov 在文件开头），同一时间只采集一个视频，和音频的采集并发分开
- 下载、合并在 `<缓存目录>/video/work` 里进行，完成后才移动到 `<缓存目录>/video`

# 5. 缓存清理

启动时和之后每天清理一次，删除 24 小时没有被访问过（`/d/music`、`/d/video` 请求或者被采集）的 mp3、视频、字幕临时目录，正在采集的跳过。

# 6. 退出

收到 Ctrl+C（SIGINT）或 SIGTERM 时正常退出，并结束正在运行的 yt-dlp、ffmpeg（Linux/macOS 下按进程组结束，不留孤儿进程）。
mp3 先转码到 `<视频ID>.tmp.mp3`，完成后才重命名成 `<视频ID>.mp3`；被强制结束时留下的临时文件在下次启动时删除。

# 7. 接口

| 接口 | 说明 |
|---|---|
| `GET /search/api?key=关键词` | 搜索，返回 `[{"name":"歌名","logo":"封面地址","videoId":"视频ID"}]`，key 为空时返回 `[]` |
| `GET /d/collect_info?videoId=xxx` | 采集状态，没有采集过时发起采集。完成时返回 `OK`，否则返回进度文字 |
| `GET /d/music?videoId=xxx` | 下载 mp3，支持 Range |
| `GET /d/video_info?videoId=xxx&type=raw\|preview` | 视频采集状态，没有采集过时发起采集（type 默认 raw）。完成时返回 `OK`，否则返回进度文字 |
| `GET /d/video?videoId=xxx&type=raw\|preview` | 下载 mp4，支持 Range，没有采集完成时返回 404 |
| `GET /d/lrc?videoId=xxx` | 字幕文本，没有时返回空字符串 |
| `GET /d/proxy?url=...` | 代理下载封面图片，只允许 YouTube 图片域名（`*.ytimg.com`） |
| `GET /set` | 上传 YouTube cookie 的页面（用 Get cookies.txt 插件从 Chrome 导出） |
| `POST /set/cookie` | 保存 YouTube cookie |
