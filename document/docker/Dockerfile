FROM ubuntu:22.04
MAINTAINER zhoulq<1064348912@qq.com>
RUN apt-get update
RUN yes|apt-get upgrade

#更新CA证书,解决HTTPS访问证书报错的BUG
RUN apt-get install -y ca-certificates
RUN update-ca-certificates

#再次更新软件元
RUN apt-get update

WORKDIR /usr/local

#####################################################################安装YT-DLP#####################################START
#https://github.com/yt-dlp/yt-dlp
#最新版yt-dlp要求Python 3.11以上,ubuntu:22.04自带的是3.10,所以用官方自带Python的独立可执行版本
ADD https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux /usr/local/bin/yt-dlp
RUN chmod a+rx /usr/local/bin/yt-dlp
#####################################################################安装YT-DLP#######################################END

#####################################################################安装DENO#####################################START
#yt-dlp需要JS运行时来解析YouTube的签名,没有的话下载时会返回403
#https://github.com/denoland/deno
ARG DENO_VERSION=2.9.8
RUN apt-get install -y unzip
ADD https://github.com/denoland/deno/releases/download/v${DENO_VERSION}/deno-x86_64-unknown-linux-gnu.zip /tmp/deno.zip
RUN unzip -o /tmp/deno.zip -d /usr/local/bin && chmod a+rx /usr/local/bin/deno && rm /tmp/deno.zip && deno --version
#####################################################################安装DENO#######################################END

#####################################################################安装FFMPEG#####################################START
#ADD https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-amd64-static.tar.xz ffmpeg-release-amd64-static.tar.xz
ADD ffmpeg-7.0.2-amd64-static.tar.xz .
RUN ln -s /usr/local/ffmpeg-7.0.2-amd64-static/ffmpeg /usr/local/bin/ffmpeg
RUN ln -s /usr/local/ffmpeg-7.0.2-amd64-static/ffprobe /usr/local/bin/ffprobe
#####################################################################安装FFMPEG#####################################END


RUN mkdir /app
WORKDIR /app

#下载程序
#ADD https://github.com/DAIRO-HY/DairoNPC/releases/download/1.0.0/dairo-npc-linux-amd64 /app
COPY dairo-music-search-linux-amd64 /app/dairo-music-search-linux-amd64

#授权可执行
RUN chmod +x dairo-music-search-linux-amd64

# 运行参数通过环境变量设置(见 docker-compose.yml):
# DAIRO_MUSIC_SEARCH_CACHE_FOLDER、DAIRO_MUSIC_SEARCH_DATA_DIR、
# DAIRO_MUSIC_SEARCH_MAX_DOWNLOAD_THREAD_COUNT、DAIRO_MUSIC_SEARCH_WEB_PORT

#启动脚本
ENTRYPOINT ["/app/dairo-music-search-linux-amd64"]
