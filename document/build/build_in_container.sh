#!/bin/bash

#在rust:1.99.1(amd64)容器里执行,由build_linux.sh调用,不要直接运行
#Linux版编译成musl静态链接的程序(和Go版一样没有依赖),在旧版本的Linux、Docker镜像(ubuntu:22.04)里都能运行
#rust:1.99.1镜像的glibc是2.41,编译成glibc版的话,在glibc比它旧的系统上无法运行
#arm64、arm、windows都是交叉编译,不需要QEMU模拟

#遇到错误立即停止
set -e

#交叉编译用的C编译器(ring需要编译C代码)
apt-get update
apt-get install -y --no-install-recommends musl-tools gcc-aarch64-linux-gnu libc6-dev-arm64-cross gcc-arm-linux-gnueabihf libc6-dev-armhf-cross gcc-mingw-w64-x86-64

rustup target add x86_64-unknown-linux-musl aarch64-unknown-linux-musl armv7-unknown-linux-musleabihf x86_64-pc-windows-gnu

export CC_x86_64_unknown_linux_musl=musl-gcc
export CC_aarch64_unknown_linux_musl=aarch64-linux-gnu-gcc
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=aarch64-linux-gnu-gcc
export CC_armv7_unknown_linux_musleabihf=arm-linux-gnueabihf-gcc
export CARGO_TARGET_ARMV7_UNKNOWN_LINUX_MUSLEABIHF_LINKER=arm-linux-gnueabihf-gcc

mkdir -p ./build

# 编译并复制到build目录
# $1:编译目标 $2:编译后的文件名 $3:发布的文件名
build(){
  cargo build --release --locked --target $1
  cp ${CARGO_TARGET_DIR:-./target}/$1/release/$2 ./build/$3
}

build x86_64-unknown-linux-musl dairo-music-search dairo-music-search-linux-amd64
build aarch64-unknown-linux-musl dairo-music-search dairo-music-search-linux-arm64
build armv7-unknown-linux-musleabihf dairo-music-search dairo-music-search-linux-arm
build x86_64-pc-windows-gnu dairo-music-search.exe dairo-music-search-win-amd64.exe
