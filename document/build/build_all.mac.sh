#!/bin/bash

#在Mac上编译macOS版程序,然后上传到GitHub(Release里已经存在同名文件时会先删除)
#需要环境变量:GITHUB_TOKEN

#遇到错误立即停止
set -e

# 进入项目根目录
cd "$(dirname "$0")/../.."

./document/build/build_mac.sh
ONLY_UPLOAD=true ./document/build/build_github.sh
