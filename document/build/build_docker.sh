#!/bin/bash

#打包并推送Docker镜像(linux/amd64),需要先执行build_linux.sh
#需要环境变量:DOCKER_USER、DOCKER_PASSWORD

#遇到错误立即停止
set -e

# 进入项目根目录
cd "$(dirname "$0")/../.."

# 获取版本号
version=$(sed -n 's/^version = "\(.*\)"/\1/p' ./Cargo.toml)

#docker用户名
docker_user=$DOCKER_USER
docker_pwd=$DOCKER_PASSWORD

#---------------------------------------上传Docker镜像-----------------------------------------
echo "正在打包Docker镜像..."
cp ./build/dairo-music-search-linux-amd64 ./document/docker/
cd ./document/docker/
docker build --no-cache --network=host -t $docker_user/dairo-music-search:$version .
rm dairo-music-search-linux-amd64

echo "$docker_pwd" | docker login -u $docker_user --password-stdin
docker push $docker_user/dairo-music-search:$version
docker logout

echo "---------------------------------------docker镜像推送完成--------------------------------------"
