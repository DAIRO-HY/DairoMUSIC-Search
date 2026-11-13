#!/bin/bash

#在macOS上编译macOS版程序(intel、M芯片),输出到build目录
#Linux上没有macOS SDK、交叉编译不了macOS版时使用(build_all.mac.sh)

#遇到错误立即停止
set -e

# 进入项目根目录
cd "$(dirname "$0")/../.."

#先创建文件夹
mkdir -p ./build

#编译后的程序名(和Cargo.toml里的包名一致)
bin_name="DairoMUSIC-Search"

#安装两个平台的编译目标
rustup target add x86_64-apple-darwin aarch64-apple-darwin

#编译到macOS intel平台
cargo build --release --locked --target x86_64-apple-darwin
cp ./target/x86_64-apple-darwin/release/$bin_name ./build/dairo-music-search-mac-amd64

#编译到macOS M平台
cargo build --release --locked --target aarch64-apple-darwin
cp ./target/aarch64-apple-darwin/release/$bin_name ./build/dairo-music-search-mac-arm64
