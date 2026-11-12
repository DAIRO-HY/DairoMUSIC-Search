#!/bin/bash

#打包并推送Docker镜像(linux/amd64、linux/arm64),需要先执行build_linux.sh
#需要环境变量:DOCKER_USER、DOCKER_PASSWORD
#可选:DOCKER_PUSH=false  只构建,不登录、不推送(用于测试Dockerfile)

#遇到错误立即停止
set -e

# 进入项目根目录
cd "$(dirname "$0")/../.."

# 获取版本号
version=$(sed -n 's/^version = "\(.*\)"/\1/p' ./Cargo.toml)

#docker用户名
docker_user=$DOCKER_USER
docker_pwd=$DOCKER_PASSWORD
image=${docker_user:-local}/dairo-music-search:$version

#---------------------------------------准备构建目录-----------------------------------------
#构建目录里只放Dockerfile和两个架构的程序,避免把整个项目(包括target目录)发送给docker
context=./build/docker
rm -rf $context
mkdir -p $context/amd64 $context/arm64
cp ./document/docker/Dockerfile $context/
cp ./build/dairo-music-search-linux-amd64 $context/amd64/DairoMUSIC-Search
cp ./build/dairo-music-search-linux-arm64 $context/arm64/DairoMUSIC-Search

#---------------------------------------构建、上传Docker镜像-----------------------------------------
echo "正在打包Docker镜像 $image ..."

#在amd64的机器上构建arm64镜像时,Dockerfile里的RUN需要通过QEMU模拟执行
docker run --privileged --rm tonistiigi/binfmt --install arm64

#Docker Desktop通过代理访问网络时,需要把代理传给builder和构建过程,否则拉取镜像、下载yt-dlp等会失败
proxy=$(docker info --format '{{.HTTPSProxy}}')
driver_opts=""
build_args=""
if [ -n "$proxy" ]; then
  [[ "$proxy" == *://* ]] || proxy="http://$proxy"
  driver_opts="--driver-opt env.http_proxy=$proxy --driver-opt env.https_proxy=$proxy"
  build_args="--build-arg http_proxy=$proxy --build-arg https_proxy=$proxy"
fi

#多平台构建需要使用docker-container驱动的builder
builder=dairo-music-search-builder
if ! docker buildx inspect $builder >/dev/null 2>&1; then
  docker buildx create --name $builder --driver docker-container $driver_opts
fi
docker buildx inspect $builder --bootstrap >/dev/null

if [ "$DOCKER_PUSH" = "false" ]; then
  #只构建,结果留在builder的缓存里
  docker buildx build --builder $builder --platform linux/amd64,linux/arm64 $build_args -t $image $context
else
  echo "$docker_pwd" | docker login -u $docker_user --password-stdin
  docker buildx build --builder $builder --platform linux/amd64,linux/arm64 $build_args -t $image --push $context
  docker logout
fi

rm -rf $context

echo "---------------------------------------docker镜像 $image 完成--------------------------------------"
