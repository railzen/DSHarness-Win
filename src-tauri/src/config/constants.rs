use std::time::Duration;

/// 捆绑的 Node.js 运行时版本（满足 v22.15.0+ / v23.8.0+ 的要求）
pub const NODE_VERSION: &str = "v22.22.0";

/// Node.js 官方下载地址
pub const NODE_BASE_URL: &str = "https://nodejs.org/dist/";

/// Node.js 镜像下载地址（npmmirror，302 重定向至 cdn.npmmirror.com）
pub const NODE_MIRROR_BASE_URL: &str = "https://npmmirror.com/mirrors/node/";

/// GitHub Release 的 ghfast.top 中转前缀（透传官方 URL，下载内容一致、
/// 仍可做 SHA-256 完整性校验），用作官方直连失败时的兜底镜像。
pub const DSH_MIRROR_PREFIX: &str = "https://ghfast.top/";

/// 捆绑的 pnpm 版本
pub const PNPM_VERSION: &str = "11.7.0";
/// pnpm 11.7.0 官方 npm tarball 的 SHA-256；升级版本时必须同步更新。
pub const PNPM_SHA256: &str = "deafa7ec98a1218b6a047289b92fbe2395c1e22d3495bb711653013218ee15ee";

/// pnpm 官方 npm registry tarball 下载地址前缀（纯 JS 发行，全平台同一 URL）
pub const PNPM_BASE_URL: &str = "https://registry.npmjs.org/pnpm/-/";

/// pnpm 镜像下载地址前缀（npmmirror registry，302 重定向至 cdn.npmmirror.com）
pub const PNPM_MIRROR_BASE_URL: &str = "https://registry.npmmirror.com/pnpm/-/";

/// Harness 服务地址与默认端口
pub const DSH_HOST: &str = "http://127.0.0.1";
/// 生产（release）默认端口
pub const DSH_PORT: u16 = 3080;
/// 开发（debug）默认端口：与生产隔离，避免 `pnpm tauri dev` 与已安装桌面端
/// 争用同一个 3080 端口冲突。
pub const DSH_DEV_PORT: u16 = 3081;

/// 官方 Harness 用户数据目录名：release 构建的 `$DSH_HOME` 默认目录（`~/.dsh`，
/// 与官方 node 安装保持一致）。
pub const DSH_HOME_DIR_NAME: &str = ".dsh";
/// 开发（debug）构建的用户数据目录名（`~/.dsh.dev`）：与生产数据目录隔离。
/// 核心（node/`dependencies/dsh`/`dependencies/pnpm`）仍共用同一份安装，但
/// 会话、档案、插件与主题等数据各自独立——`pnpm tauri dev` 与已安装桌面端
/// 同时运行时互不干扰，也不会互相污染对方的会话数据。
pub const DSH_HOME_DEV_DIR_NAME: &str = ".dsh.dev";

/// 安装目录与 CLI 入口（相对安装目录）
pub const DSH_CORE_DIR: &str = "dsh";
pub const DSH_ENTRY_RELATIVE: &str = "node_modules/@deepseek-ai/dsh/lib/bin.js";
pub const DSH_MANIFEST_RELATIVE: &str = "package.json";

/// pnpm 安装目录与 CLI 入口（相对安装目录）
pub const PNPM_CORE_DIR: &str = "pnpm";
pub const PNPM_ENTRY_RELATIVE: &str = "bin/pnpm.cjs";

/// 旧版数据目录名：迁移前 $DSH_HOME 位于 `{app_data}/data/dsh`，
/// 现仅用于 legacy 路径识别（见 service::migrate）。新 $DSH_HOME = 官方 `~/.dsh`。
pub const DSH_DATA_DIR_NAME: &str = "dsh";

/// 简单 Store 持久化
pub const STORE_DAT_FILE: &str = ".store.dat";
/// 开发（debug）构建的 Store 持久化文件名：与生产隔离，避免端口、installed、
/// active_core 等设置跨版本互写（生产默认 3080、开发默认 3081，共用一份
/// store 会让两边端口一路漂移并相互污染状态）。
pub const STORE_DAT_DEV_FILE: &str = ".store.dev.dat";
pub const STORE_SETTING_KEY: &str = "setting";
/// Store 中记录主窗口几何（位置/大小/最大化）的键
pub const STORE_WINDOW_STATE_KEY: &str = "window_state";

/// 健康检查超时
pub const HEALTH_CHECK_TIMEOUT: Duration = Duration::from_secs(5);
