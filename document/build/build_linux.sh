#!/bin/bash

#编译Linux(amd64、arm64、arm)和Windows(amd64)版程序,输出到build目录
#在rust:1.99.1容器里交叉编译(document/build/build_in_container.sh),只需要安装docker

#遇到错误立即停止
set -e

# 当前 sh 文件所在目录
script_dir="$(cd "$(dirname "$0")" && pwd)"

# 项目目录
project_root="$(cd "$script_dir/../.." && pwd)"

#挂载到编译容器的目录一定是宿主机上的绝对路径
#在Jenkins容器里调用docker时,当前目录是Jenkins容器里的路径,需要通过环境变量指定宿主机上的路径
#也可以指定Docker命名卷的卷名(document/docker-build/build-linux.sh使用这种方式)
host_project_dir=${HOST_PROJECT_DIR:-$project_root}
host_cargo_dir=${HOST_CARGO_DIR:-$HOME/.cargo}
host_cargo_registry=${HOST_CARGO_REGISTRY:-$host_cargo_dir/registry}

#容器不会继承docker的代理设置(比如Docker Desktop通过代理访问网络),需要手动传入,否则apt、cargo无法联网
proxy_opts=""
proxy=$(docker info --format '{{.HTTPSProxy}}')
if [ -n "$proxy" ]; then
  [[ "$proxy" == *://* ]] || proxy="http://$proxy"
  proxy_opts="-e http_proxy=$proxy -e https_proxy=$proxy"
fi

#编译目录放在Docker命名卷里,不放在挂载的项目目录下:
#Docker Desktop挂载宿主机目录的读写很慢,大量编译中间文件写在挂载目录里时编译会卡住
#命名卷在Docker虚拟机内部,读写快,而且下次编译可以复用
target_volume=${TARGET_VOLUME:-dairo-music-search-target}
docker run --platform linux/amd64 --rm $proxy_opts \
  -v $host_project_dir:/home/rust/src \
  -v $host_cargo_registry:/usr/local/cargo/registry \
  -v $target_volume:/home/rust/target \
  -e CARGO_TARGET_DIR=/home/rust/target \
  -w /home/rust/src rust:1.99.1 \
  ./document/build/build_in_container.sh

ls -l $project_root/build/
