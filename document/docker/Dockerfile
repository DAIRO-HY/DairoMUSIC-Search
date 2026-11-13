#运行镜像,支持 linux/amd64、linux/arm64 两个架构(由 document/build/build_docker.sh 用 docker buildx 构建)
#构建目录的结构:
#  Dockerfile
#  amd64/DairoMUSIC-Search    amd64版程序(build/dairo-music-search-linux-amd64)
#  arm64/DairoMUSIC-Search    arm64版程序(build/dairo-music-search-linux-arm64)
#程序在rust:1.99.1(Debian 13 trixie,glibc 2.41)里编译,运行环境也使用trixie,glibc版本一致(和DairoMUSIC相同)
#用旧版本的系统(比如bookworm的glibc 2.36、ubuntu:22.04的2.35)会因为缺少新版本glibc的函数而无法运行
FROM debian:trixie-slim
MAINTAINER zhoulq<1064348912@qq.com>

#构建的目标架构(amd64、arm64),由 docker buildx 自动传入
ARG TARGETARCH

#tini:作为1号进程转发退出信号、回收僵尸进程(yt-dlp、ffmpeg)
#ca-certificates:访问https;tzdata:docker-compose里设置的TZ时区
#curl、unzip、xz-utils:下载、解压yt-dlp、deno、ffmpeg
RUN apt-get update \
    && apt-get install -y --no-install-recommends tini ca-certificates tzdata curl unzip xz-utils \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/local

#####################################################################安装YT-DLP#####################################START
#https://github.com/yt-dlp/yt-dlp
#用官方自带Python的独立可执行版本,不需要安装Python
RUN case "$TARGETARCH" in \
      amd64) file=yt-dlp_linux ;; \
      arm64) file=yt-dlp_linux_aarch64 ;; \
      *) echo "不支持的架构:$TARGETARCH" && exit 1 ;; \
    esac \
    && curl -fsSL -o /usr/local/bin/yt-dlp https://github.com/yt-dlp/yt-dlp/releases/latest/download/$file \
    && chmod a+rx /usr/local/bin/yt-dlp
#####################################################################安装YT-DLP#######################################END

#####################################################################安装DENO#####################################START
#yt-dlp需要JS运行时来解析YouTube的签名,没有的话下载时会返回403
#https://github.com/denoland/deno
ARG DENO_VERSION=2.9.8
RUN case "$TARGETARCH" in \
      amd64) target=x86_64-unknown-linux-gnu ;; \
      arm64) target=aarch64-unknown-linux-gnu ;; \
      *) echo "不支持的架构:$TARGETARCH" && exit 1 ;; \
    esac \
    && curl -fsSL -o /tmp/deno.zip https://github.com/denoland/deno/releases/download/v${DENO_VERSION}/deno-$target.zip \
    && unzip -o /tmp/deno.zip -d /usr/local/bin \
    && chmod a+rx /usr/local/bin/deno \
    && rm /tmp/deno.zip \
    && deno --version
#####################################################################安装DENO#######################################END

#####################################################################安装FFMPEG#####################################START
#静态编译版(https://johnvansickle.com/ffmpeg/),release链接始终指向最新的正式版本(目前是7.0.2)
#升级到7.x以外的版本时构建会失败,需要确认兼容后再调整版本检查;也可以用 --build-arg FFMPEG_URL=... 指定其他下载地址(只构建单个架构时使用)
ARG FFMPEG_URL=""
RUN url="$FFMPEG_URL" \
    && if [ -z "$url" ]; then url=https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-$TARGETARCH-static.tar.xz; fi \
    && curl -fsSL -o /tmp/ffmpeg.tar.xz "$url" \
    && mkdir -p /usr/local/ffmpeg \
    && tar -xJf /tmp/ffmpeg.tar.xz -C /usr/local/ffmpeg --strip-components=1 \
    && rm /tmp/ffmpeg.tar.xz \
    && ln -s /usr/local/ffmpeg/ffmpeg /usr/local/bin/ffmpeg \
    && ln -s /usr/local/ffmpeg/ffprobe /usr/local/bin/ffprobe \
    && ffmpeg -version | head -1 \
    && ffmpeg -version | head -1 | grep -q "^ffmpeg version 7\."
#####################################################################安装FFMPEG#####################################END


RUN mkdir /app
WORKDIR /app

#复制对应架构的程序
COPY $TARGETARCH/DairoMUSIC-Search /app/DairoMUSIC-Search

#授权可执行
RUN chmod +x DairoMUSIC-Search

# 运行参数通过环境变量设置(见 docker-compose.yml):
# DAIRO_MUSIC_SEARCH_HOST、DAIRO_MUSIC_SEARCH_WEB_PORT、DAIRO_MUSIC_SEARCH_CACHE_FOLDER、DAIRO_MUSIC_SEARCH_DATA_DIR、
# DAIRO_MUSIC_SEARCH_MAX_DOWNLOAD_THREAD_COUNT

#解决控制台输出中文乱码
ENV LANG=C.UTF-8

#启动脚本
ENTRYPOINT ["/usr/bin/tini", "--", "/app/DairoMUSIC-Search"]
