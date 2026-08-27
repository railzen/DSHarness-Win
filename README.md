<p align="center">
  <a href="https://github.com/railzen/deepseek-harness-win">
    <img src="public/favicon.svg" width="96" alt="DeepSeek Harness Desktop" />
  </a>
</p>

<h1 align="center">DeepSeek Harness 桌面版</h1>

<p align="center">
  在桌面上一键运行 <a href="https://github.com/deepseek-ai/deepseek-harness">DeepSeek Harness</a> ——<br />
  无需 Node.js、无需 pnpm、无需 Docker，下载即用。
</p>

<p align="center">
  <a href="https://github.com/railzen/deepseek-harness-win/releases">
    <img src="https://img.shields.io/github/v/release/railzen/deepseek-harness-win?style=flat-square&label=release&color=4D6BFE" alt="Release" />
  </a>
  <img src="https://img.shields.io/github/downloads/railzen/deepseek-harness-win/total?style=flat-square&label=downloads&color=4D6BFE" alt="Downloads" />
  <img src="https://img.shields.io/github/stars/railzen/deepseek-harness-win?style=flat-square&label=stars&color=4D6BFE" alt="Stars" />
  <img src="https://img.shields.io/github/license/railzen/deepseek-harness-win?style=flat-square&label=license&color=4D6BFE" alt="MIT License" />
  <img src="https://img.shields.io/badge/Windows%20%7C%20macOS%20%7C%20Linux-black?style=flat-square" alt="Windows | macOS | Linux" />
</p>

<p align="center">
  <samp><a href="./README.en.md">English</a> · <strong>中文</strong></samp>
</p>

<p align="center">
  <img src="./docs/images/hero-zh.png" width="100%" alt="DSH Desktop 中文宣传横幅" />
</p>

<table>
  <tr>
    <td><a href="docs/PREVIEW.md"><img src="./docs/images/previews/preview-1.png" alt="preview 1" /></a></td>
    <td><a href="docs/PREVIEW.md"><img src="/docs/images/previews/preview-2.png" alt="preview 2" /></a></td>
    <td><a href="docs/PREVIEW.md"><img src="/docs/images/previews/preview-4.png" alt="preview 4" /></a></td>
    <td><a href="docs/PREVIEW.md"><img src="/docs/images/previews/preview-5.png" alt="preview 5" /></a></td>
  </tr>
</table>

- 🧩 **原生上游** — 仅安装 DeepSeek 官方 Release tag 对应的 Harness 与上游插件。
- 🪶 **原生轻量** — Tauri 2 外壳（非 Electron）：更小的安装包、更低的内存占用、原生窗口。
- ⌨️ **命令行集成** — 安装自动注册 `dsh` 命令，新开终端即用；不覆盖你已有 shell 配置。
- 🚀 **自更新** — 应用内更新，不需要在重新下载；

## 快速开始

从 [Releases](https://github.com/railzen/deepseek-harness-win/releases) 下载对应平台安装包，安装后启动即可。

首次运行会下载 Node 运行时与 Harness 内核（如已经安装 `dsh` ，则使用安装版本），随后直接进入 `http://127.0.0.1:3080` 的 Harness 界面；此后完全本地运行，无需联网。

**系统要求：** Windows 10+ · macOS 10.15+ · Linux（AppImage / .deb）· 首次运行需要网络

> **Linux Wayland 注意（PikaOS / GNOME Wayland / Ubuntu 22.04+）：** AppImage 在 Wayland 下可能因 WebKitGTK 黑屏/崩溃，应用已自动处理常见情形。 <details><summary>若仍黑屏/崩溃：</summary><br>**改用 `.deb`**（已验证 PikaOS 4 Wayland），或手动 `WEBKIT_DISABLE_COMPOSITING_MODE=1 WEBKIT_DISABLE_DMABUF_RENDERER=1 GDK_BACKEND=x11 ./AppImage`。图标不显示时，将应用内 `hicolor` 图标复制到 `~/.local/share/icons` 并运行 `update-desktop-database`。<br></details>

## 交流

<img width="360" height="566" alt="image" src="https://github.com/user-attachments/assets/598308b5-681d-4514-a8d7-a36810fa8636" />


## 开发

想参与开发？参见 [docs/DEVELOPMENT.zh.md](./docs/DEVELOPMENT.zh.md)。

## 工作原理

```text
┌──────────────────────────────────────────────┐
│ Tauri WebView (React)                        │
│   安装状态机 → 下载进度 → iframe              │
│   加载 dsh Web 界面 + 侧边栏控制              │
└──────────────────────┬───────────────────────┘
                       │ invoke 命令 + 事件
┌──────────────────────┴───────────────────────┐
│ Tauri Rust 后端                              │
│   service/download  安装器 + 解压            │
│   service/core      Harness 核心多版本管理   │
│   service/profile   dsh 档案管理             │
│   service/cli       dsh 命令 shim + PATH     │
│   service/update    桌面端自更新             │
│   service/workflow  dsh 进程生命周期         │
│   task              dsh 健康检查             │
└──────┬───────────────────────────┬───────────┘
       │                           │
  runtime/ (Node.js v22.22.0)   dependencies/dsh/ (官方版本)
       └─────────────┬─────────────┘
                     ▼
   dsh --profile <档案> --host 127.0.0.1 --port 3080
                     │  DSH_HOME=~/.dsh
                     ▼
        http://127.0.0.1:3080/  ← 内嵌界面
```

Harness 根据 [DeepSeek 官方 Release](https://github.com/deepseek-ai/deepseek-harness/releases) 的 tag 安装对应的 `@deepseek-ai/dsh` 官方版本。桌面端自更新只查询本项目 Release；仓库尚无 Release 时视为已是最新版本。

## 说明

> [!WARNING]
> **开发预览** — 上游 `dsh` 仍在快速迭代，存在破坏性变更；本项目同步跟随。

> [!NOTE]
> **安全声明** — `dsh` 具备本地代码执行能力。仅供学习 / 研究 / 测试，请在可信、隔离的环境中使用。

## 相关项目

- [deepseek-harness](https://github.com/deepseek-ai/deepseek-harness) — 上游 `dsh` agent 平台
- [n8n-desktop](https://github.com/tangtao646/n8n-desktop) — 参考实现

## License

[MIT](./LICENSE)，附加[非商用条款](./LICENSE.details) © deepseek-harness-desktop contributors
