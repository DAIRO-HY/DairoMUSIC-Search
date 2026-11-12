#运行镜像,支持 linux/amd64、linux/arm64 两个架构(由 document/build/build_docker.sh 用 docker buildx 构建)
#构建目录的结构:
#  Dockerfile
#  amd64/DairoMUSIC-Search    amd64版程序(build/dairo-music-search-linux-amd64)
#  arm64/DairoMUSIC-Search    arm64版程序(build/dairo-music-search-linux-arm64)
FROM ubuntu:22.04
MAINTAINER zhoulq<1064348912@qq.com>

#构建的目标架构(amd64、arm64),由 docker buildx 自动传入
ARG TARGETARCH

RUN apt-get update
RUN yes|apt-get upgrade

#更新CA证书,解决HTTPS访问证书报错的BUG
RUN apt-get install -y ca-certificates
RUN update-ca-certificates

#下载、解压用的工具
RUN apt-get install -y curl unzip xz-utils

WORKDIR /usr/local

#####################################################################安装YT-DLP#####################################START
#https://github.com/yt-dlp/yt-dlp
#最新版yt-dlp要求Python 3.11以上,ubuntu:22.04自带的是3.10,所以用官方自带Python的独立可执行版本
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

#启动脚本
ENTRYPOINT ["/app/DairoMUSIC-Search"]
