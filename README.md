# DeepSeek Harness 桌面版

在 Windows、macOS 和 Linux 桌面上一键运行 [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness)。这是一个基于 Tauri 2 的桌面封装，提供本地窗口、启动管理和 `dsh` 命令行入口。

## 当前版本：v0.0.2

- 初始版本，作为上游 DeepSeek Harness 的桌面端封装。
- 用于降低上游频繁更新对使用体验的影响。
- 移除 DSH 独立安装机制，DSH 改为全局安装。
- 移除插件管理功能，仅保留 DeepSeek 官方 Release 对应的上游内容。
- 当前版本以跟随上游、修复缺陷为主，暂不计划增加额外功能。

## 快速开始

从 [Releases](https://github.com/railzen/deepseek-harness-win/releases) 下载对应平台的安装包，安装后启动即可。

首次启动需要网络，并会准备运行环境；随后在本机启动 Harness，默认地址为 `http://127.0.0.1:3080`。运行过程中无需持续联网。

仅支持 Windows 平台

## 相关项目以及License

- [deepseek-harness-desktop](https://github.com/dsh-tauri-desk/deepseek-harness-desktop) — 上游 Harness 项目
- [MIT](./LICENSE)，额外遵循上游[非商用条款](./LICENSE.details) 

