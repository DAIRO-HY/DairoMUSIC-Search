#!/bin/bash

#编译Linux、Windows、macOS版程序 -> 发布到GitHub -> 推送Docker镜像
#macOS版需要macOS SDK(见build_cross.sh),没有时跳过,可以在Mac上执行build_all.mac.sh补传

#遇到错误立即停止
set -e

# 进入项目根目录
cd "$(dirname "$0")/../.."

./document/build/build_linux.sh
./document/build/build_cross.sh
./document/build/build_github.sh
./document/build/build_docker.sh
