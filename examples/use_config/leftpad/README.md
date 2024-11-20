# 蓝牙双模左手Pad

RMK 使用Nrf52840作为主控

## 左手Pad

镜像Pad和常规Pad镜像

以下是如何使.uf2 固件在 RMK 中工作的步骤：

1. 获取cargo-make工具：
   ```shell
   cargo install --force cargo-make
   ```
2. 编译RMK并得到.uf2：
   ```shell
   cargo make uf2 --release
   ```
