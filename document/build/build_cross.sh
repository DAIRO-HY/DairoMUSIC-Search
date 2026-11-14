#!/bin/bash

#在Linux(amd64)上交叉编译Windows、macOS的amd64和arm64版本(和DairoMUSIC相同),输出到build目录
#Windows:不需要额外的文件
#macOS:需要macOS SDK,在Mac上执行pack_macos_sdk.sh打包,复制到这台机器解压到 ~/develop_tool/MacOSX.sdk
#      没有SDK时跳过macOS版本(Windows版本照常编译);也可以在Mac上执行build_all.mac.sh编译并补传

#遇到错误立即停止
set -e

# 当前 sh 文件所在目录
script_dir="$(cd "$(dirname "$0")" && pwd)"

# 项目目录
project_root="$(cd "$script_dir/../.." && pwd)"

#先创建文件夹
mkdir -p $project_root/build

#编译后的程序名(和Cargo.toml里的包名一致)
bin_name="DairoMUSIC-Search"

#挂载到编译容器的目录一定是宿主机上的绝对路径
#在Jenkins容器里调用docker时,当前目录是Jenkins容器里的路径,需要通过环境变量指定宿主机上的路径
#也可以指定Docker命名卷的卷名(document/docker-build/build-linux.sh使用这种方式)
host_project_dir=${HOST_PROJECT_DIR:-$project_root}
host_cargo_dir=${HOST_CARGO_DIR:-$HOME/.cargo}
host_cargo_registry=${HOST_CARGO_REGISTRY:-$host_cargo_dir/registry}

#macOS SDK(解压后的MacOSX.sdk目录),默认 ~/develop_tool/MacOSX.sdk,不存在时跳过macOS版本
#MACOS_SDK:当前环境里的路径,用来判断是否存在
#HOST_MACOS_SDK:宿主机上的路径,挂载到编译容器(在Jenkins容器里执行时两者不同)
macos_sdk=${MACOS_SDK:-$HOME/develop_tool/MacOSX.sdk}
host_macos_sdk=${HOST_MACOS_SDK:-$macos_sdk}

#构建、编译过程不会继承docker的代理设置,需要手动传入,否则下载zig、cargo-zigbuild、依赖会失败
proxy=$(docker info --format '{{.HTTPSProxy}}')
proxy_args=""
proxy_opts=""
if [ -n "$proxy" ]; then
  [[ "$proxy" == *://* ]] || proxy="http://$proxy"
  proxy_args="--build-arg http_proxy=$proxy --build-arg https_proxy=$proxy"
  proxy_opts="-e http_proxy=$proxy -e https_proxy=$proxy"
fi

#构建交叉编译环境的镜像(只有第一次和Dockerfile修改之后才会真正构建)
#使用宿主机的网络:有的服务器上docker默认网络(bridge)里解析不了域名,构建时会报 Could not resolve host
cross_image=dairo-music-search-cross:1.99.1
docker build --network host --platform linux/amd64 $proxy_args -t $cross_image $script_dir/cross_dockerfile

#编译目录放在Docker命名卷里(原因见build_linux.sh)
target_volume=${TARGET_VOLUME:-dairo-music-search-target}

# 编译并复制到build目录(编译目录在命名卷里,只能在容器里复制)
# $1:编译目标 $2:编译后的文件名 $3:发布的文件名
cross_build(){
  sdk_opts=""
  if [[ "$1" == *-apple-darwin ]]; then
    #链接时zig会把-L、-F的路径加上SDK目录作为前缀,所以这里写SDK里的相对路径
    target_env=$(echo "$1" | tr 'a-z-' 'A-Z_')
    sdk_opts="-v $host_macos_sdk:/opt/MacOSX.sdk:ro -e SDKROOT=/opt/MacOSX.sdk"
    sdk_opts="$sdk_opts -e CARGO_TARGET_${target_env}_RUSTFLAGS=-Lnative=/usr/lib"
  fi
  docker run --platform linux/amd64 --rm $proxy_opts \
    -v $host_project_dir:/home/rust/src \
    -v $host_cargo_registry:/usr/local/cargo/registry \
    -v $target_volume:/home/rust/target \
    -e CARGO_TARGET_DIR=/home/rust/target/cross \
    $sdk_opts \
    -w /home/rust/src $cross_image \
    sh -c "cargo zigbuild --release --locked --target $1 && cp /home/rust/target/cross/$1/release/$2 ./build/$3"
}

#发布的文件名保持 dairo-music-search-<平台> 不变(DairoMUSIC 内置安装时按这个文件名下载)
#Windows
cross_build x86_64-pc-windows-gnu $bin_name.exe dairo-music-search-win-amd64.exe
cross_build aarch64-pc-windows-gnullvm $bin_name.exe dairo-music-search-win-arm64.exe

#macOS
if [ -f "$macos_sdk/usr/lib/libSystem.tbd" ]; then
  cross_build x86_64-apple-darwin $bin_name dairo-music-search-mac-amd64
  cross_build aarch64-apple-darwin $bin_name dairo-music-search-mac-arm64
else
  echo "没有找到macOS SDK:$macos_sdk,跳过macOS版本(在Mac上执行pack_macos_sdk.sh打包SDK,解压到这个目录)"
fi

ls -l $project_root/build/
