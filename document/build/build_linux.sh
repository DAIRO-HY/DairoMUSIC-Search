#!/bin/bash

#编译Linux版程序(amd64、arm64),输出到build目录
#和DairoMUSIC一样,在官方rust:1.99.1镜像(Debian 13,glibc 2.41)里编译成glibc动态链接的程序,arm64通过QEMU模拟编译
#编译出的程序需要glibc 2.39以上,旧版本的Linux请用Docker运行

#遇到错误立即停止
set -e

#前提条件 安装 QEMU binfmt,目的:在amd64位linux系统上编译arm64位linux二进制文件
docker run --privileged --rm tonistiigi/binfmt --install arm64

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

#容器不会继承docker的代理设置(比如Docker Desktop通过代理访问网络),需要手动传入,否则cargo无法联网
proxy_opts=""
proxy=$(docker info --format '{{.HTTPSProxy}}')
if [ -n "$proxy" ]; then
  [[ "$proxy" == *://* ]] || proxy="http://$proxy"
  proxy_opts="-e http_proxy=$proxy -e https_proxy=$proxy"
fi

#编译目录放在Docker命名卷里,不放在挂载的项目目录下:
#Docker Desktop挂载宿主机目录的读写很慢,大量编译中间文件写在挂载目录里时编译会卡住
#命名卷在Docker虚拟机内部,读写快,而且下次编译可以复用(不同架构用不同的子目录)
target_volume=${TARGET_VOLUME:-dairo-music-search-target}

# 编译并复制到build目录(编译目录在命名卷里,只能在容器里复制)
# $1:架构(amd64、arm64)
docker_rust_build(){
  docker run --platform "linux/$1" --rm $proxy_opts \
    -v $host_project_dir:/home/rust/src \
    -v $host_cargo_registry:/usr/local/cargo/registry \
    -v $target_volume:/home/rust/target \
    -e CARGO_TARGET_DIR=/home/rust/target/linux-$1 \
    -w /home/rust/src rust:1.99.1 \
    sh -c "cargo build --release --locked && cp /home/rust/target/linux-$1/release/$bin_name ./build/dairo-music-search-linux-$1"
}

#发布的文件名保持 dairo-music-search-<平台> 不变(DairoMUSIC 内置安装时按这个文件名下载)
docker_rust_build amd64
docker_rust_build arm64

ls -l $project_root/build/
