# Superherdr

Superherdr 是基于 [Herdr](https://github.com/herdrdev/herdr) 的开源终端项目，内置 Focus（专注）和 Snooze（暂时隐藏）。它保留 Herdr 的原生终端运行时，并跟进上游。

目前请从源码安装；上游 Herdr 的安装器不会安装Superherdr。安装 Rust 和 Zig 0.15.2 后，在本仓库中运行：

```sh
cargo install --path . --locked
superherdr
```

可执行文件名为 `superherdr`，不会覆盖 `herdr`。Unix 发布构建使用 `~/.config/superherdr` 和 `~/.local/state/superherdr`，调试构建使用 `superherdr-dev`。现有 Herdr 会话和配置不会迁移或停止。

为防止上游版本覆盖Superherdr，二进制自动更新已禁用。`HERDR_*` 环境变量、套接字文件名和 API 标识符保持兼容。

完整安装、SSH、兼容性说明见 [English README](README.md)。本项目保留 Herdr 的 [Apache 2.0 许可证](LICENSE)及版权声明。
