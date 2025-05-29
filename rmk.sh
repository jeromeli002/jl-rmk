#!/bin/bash
echo "更新系统软件包列表..."
sudo apt update

# 1. 安装必要的软件包
echo "安装必要的软件包: curl, git, rustup, clang..."
sudo apt install -y curl git clang

# 2. 安装 Rust (使用 rustup)
echo "安装 Rust..."
  curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
echo "Rust 安装成功。"

# 3. 添加 nRF52840 目标库
echo "添加 Rust 目标: thumbv7em-none-eabihf..."
    rustup target add thumbv7em-none-eabihf
echo "Rust 目标添加成功。"

# 4. 安装 cargo-make
echo "安装 cargo-make..."
    cargo install --force cargo-make
echo "cargo-make 安装成功。"

echo "RMK固件编译环境设置完成！"
echo "编译固件：cargo build"
echo "固件生成：cargo make uf2 --release"
echo "清除文件： cargo clean"
echo "查找代码： grep -r '你要查找的代码片段'"

