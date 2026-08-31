use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

const DSH_MAX_LOG_BYTES: u64 = 5 * 1024 * 1024;
const DSH_MAX_BACKUPS: usize = 3;
static DSH_LOG_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static AUTHENTICATED_SERVICE_URL: OnceLock<Mutex<Option<(u16, String)>>> = OnceLock::new();
fn dsh_log_lock() -> &'static Mutex<()> {
    DSH_LOG_LOCK.get_or_init(|| Mutex::new(()))
}

fn authenticated_service_url_lock() -> &'static Mutex<Option<(u16, String)>> {
    AUTHENTICATED_SERVICE_URL.get_or_init(|| Mutex::new(None))
}

/// 从新版 dsh 的就绪输出中提取仅指向本机的认证地址。
///
/// token 是每次进程启动随机生成的，不能从配置推导；同时严格限制协议、主机、
/// 路径与查询参数，避免把未来日志里的任意外部 URL 交给 WebView 或系统浏览器。
fn parse_authenticated_service_url(line: &str) -> Option<(u16, String)> {
    let raw = line.strip_prefix("dsh web: ")?.split_whitespace().next()?;
    let url = reqwest::Url::parse(raw).ok()?;
    if url.scheme() != "http"
        || url.host_str() != Some("127.0.0.1")
        || url.path() != "/"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return None;
    }
    let port = url.port()?;
    let mut query = url.query_pairs();
    let (name, token) = query.next()?;
    if name != "token" || token.is_empty() || query.next().is_some() {
        return None;
    }
    Some((port, url.to_string()))
}

/// 记录当前 dsh 进程公布的认证 URL；旧版无 token 的输出保持原有探测流程。
fn record_authenticated_service_url(line: &str) {
    let Some(value) = parse_authenticated_service_url(line) else {
        return;
    };
    let mut guard = authenticated_service_url_lock()
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    *guard = Some(value);
}

/// 返回指定端口当前有效的认证 URL，避免端口切换后复用旧进程凭据。
pub(super) fn authenticated_service_url(port: u16) -> Option<String> {
    authenticated_service_url_lock()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .filter(|(recorded_port, _)| *recorded_port == port)
        .map(|(_, url)| url.clone())
}

/// 把当前认证 URL 的主机替换为桌面 WebView 使用的同站点回环域名。
///
/// token 由 dsh 进程校验、并不绑定 URL 主机；端口与其余 URL 组件保持不变。
/// 调用方必须同时通过 `--trusted-host` 把该域名加入 dsh 的 Host/API 信任边界。
pub(super) fn authenticated_service_url_for_host(port: u16, host: &str) -> Option<String> {
    let raw = authenticated_service_url(port)?;
    let mut url = reqwest::Url::parse(&raw).ok()?;
    url.set_host(Some(host)).ok()?;
    Some(url.to_string())
}

/// 新进程启动或旧进程退出时立即作废上一次的启动 token。
pub(super) fn clear_authenticated_service_url() {
    let mut guard = authenticated_service_url_lock()
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    *guard = None;
}

/// 构造仅用于回环地址探测的 HTTP 客户端。
///
/// 生命周期探测访问的是本机 dsh，不能继承 `HTTP_PROXY` / `ALL_PROXY`：部分代理
/// 不尊重回环地址直连，或应用进程没有 `NO_PROXY`，会把健康检查转发到外部代理，
/// 造成端口已经监听但持续误报未就绪。
pub(super) fn loopback_http_client(timeout: Duration) -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .no_proxy()
        .timeout(timeout)
        .build()
}

/// 客户端插件 bundle 探测地址。
///
/// SPA `/` 在 webServer 绑定后立刻 200，此时连接桥与 Loader 图往往还没就绪；
/// WebView 若在这个窗口加载，会永久停在官方 boot 页 “Loading plugins…”。
/// 必须等到真实 JS bundle（而非 HTML fallback）可取，才视为可挂载 iframe。
pub(super) fn health_probe_plugin_urls(port: u16) -> Vec<String> {
    vec![
        format!("http://127.0.0.1:{port}/plugins/@deepseek-ai/dsh-client-ui-layout/client.js"),
        format!("http://127.0.0.1:{port}/plugins/@deepseek-ai/dsh-client-runtime/client.js"),
    ]
}

/// 判断健康检查响应是不是可用的插件 bundle。
///
/// 未知 `/plugins/...` 路径会被 SPA fallback 成 `index.html`（仍是 200），
/// 绝不能当成插件已就绪。
pub(super) fn looks_like_plugin_bundle(ok_status: bool, body: &str) -> bool {
    if !ok_status {
        return false;
    }
    let trimmed = body.trim_start();
    if trimmed.is_empty() {
        return false;
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("<!doctype") || lower.starts_with("<html") {
        return false;
    }
    true
}

/// 检查 Harness 是否真正在运行（探测指定端口，随配置端口联动）
pub async fn is_dsh_running(port: u16) -> bool {
    let client = loopback_http_client(Duration::from_secs(2)).ok(); // 将 Result 转为 Option

    // 如果 client 创建失败，直接返回 false
    let client = match client {
        Some(c) => c,
        None => return false,
    };

    let url = format!("{}/", crate::config::get_dsh_service_url(port));

    // 发送请求并判断是否就绪
    let check_status = async {
        let resp = client.get(&url).send().await.ok()?;
        if resp.status() != reqwest::StatusCode::OK {
            return None;
        }
        Some(true)
    };

    check_status.await.unwrap_or(false)
}

/// 检查指定端口是否被占用（通过尝试连接来判断）
pub fn is_port_in_use(port: u16) -> bool {
    // 以实际绑定结果判断，能够识别“已绑定但尚未 listen”的占用状态。
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
    TcpListener::bind(addr).is_err()
}

/// 在独立线程中读取子进程的输出，同时写入日志文件
///
/// # 参数
/// - `stdout`: 子进程的标准输出
/// - `stderr`: 子进程的标准错误输出
/// - `log_path`: 前端日志面板读取的日志文件
pub fn spawn_output_readers<R1, R2>(stdout: Option<R1>, stderr: Option<R2>, log_path: PathBuf)
where
    R1: Read + Send + 'static,
    R2: Read + Send + 'static,
{
    // 在独立线程中读取 stdout
    if let Some(stdout) = stdout {
        let log_path = log_path.clone();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        record_authenticated_service_url(&line);
                        log::info!(target: "dsh", "{}", line);
                        append_log(&log_path, &line);
                    }
                    Err(e) => {
                        log::error!("Failed to read dsh stdout: {}", e);
                        break;
                    }
                }
            }
        });
    }

    // 在独立线程中读取 stderr
    if let Some(stderr) = stderr {
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        log::warn!(target: "dsh", "{}", line);
                        append_log(&log_path, &line);
                    }
                    Err(e) => {
                        log::error!("Failed to read dsh stderr: {}", e);
                        break;
                    }
                }
            }
        });
    }
}

fn append_log(log_path: &PathBuf, line: &str) {
    // 与 `logger` 的 `desktop.log` / `desktop.frontdesk.log` 保持一致：5MiB × 3 轮转
    let _guard = dsh_log_lock().lock().unwrap_or_else(|e| e.into_inner());
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
    {
        let _ = writeln!(file, "{}", line);
        let _ = file.flush();
    }
    // 超阈值则按大小轮转（与启动次轮转 `rotate_service_log` 互补，避免单次运行无限增长）
    if let Ok(meta) = std::fs::metadata(log_path) {
        if meta.len() > DSH_MAX_LOG_BYTES {
            let _ = std::fs::remove_file(indexed_log_path(log_path, DSH_MAX_BACKUPS));
            for i in (1..DSH_MAX_BACKUPS).rev() {
                let from = indexed_log_path(log_path, i);
                let to = indexed_log_path(log_path, i + 1);
                if from.exists() {
                    let _ = std::fs::remove_file(&to);
                    let _ = std::fs::rename(&from, &to);
                }
            }
            if log_path.exists() {
                let _ = std::fs::rename(log_path, indexed_log_path(log_path, 1));
            }
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(log_path);
        }
    }
}

/// 轮转日志文件名：`dsh-web.log`（index 0）、`dsh-web.log.1`、`dsh-web.log.2`……
fn indexed_log_path(log_path: &PathBuf, index: usize) -> PathBuf {
    if index == 0 {
        return log_path.clone();
    }
    let mut name = log_path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{}", index));
    log_path.with_file_name(name)
}

/// 每次启动服务前轮转日志，只保留最近 `keep` 次启动产生的日志文件。
///
/// 把当前 `dsh-web.log` 依次后退为 `.1`、`.2`……，超过保留上限的最老文件
/// 直接删除，再以空文件重新记录本次启动日志。这样磁盘上始终只保留最近
/// `keep` 次 dsh 启动的日志，避免单文件随多次启动无限增长。
pub fn rotate_service_log(log_path: &PathBuf, keep: usize) {
    if keep == 0 {
        let _ = std::fs::remove_file(log_path);
        return;
    }
    // 1) 删除超过保留上限的最老文件（它会被顶上来的文件覆盖且无处安放）
    let _ = std::fs::remove_file(&indexed_log_path(log_path, keep - 1));
    // 2) 从次老到次新依次后移，为本次启动腾出位置
    for i in (1..keep).rev() {
        let from = indexed_log_path(log_path, i);
        let to = indexed_log_path(log_path, i + 1);
        if from.exists() {
            let _ = std::fs::remove_file(&to);
            let _ = std::fs::rename(&from, &to);
        }
    }
    // 3) 当前日志后移为 `.1`，重新开始本次记录
    if log_path.exists() {
        let _ = std::fs::rename(log_path, indexed_log_path(log_path, 1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(path: &PathBuf, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    /// 模拟连续 5 次启动，验证磁盘上始终只保留最近 `keep` 份日志，
    /// 且每次启动都会新建当前日志文件。
    #[test]
    fn rotate_keeps_only_last_three_starts() {
        let dir = std::env::temp_dir().join(format!("dsh_rotate_test_{}", std::process::id()));
        let log = dir.join("dsh-web.log");
        let _ = fs::remove_dir_all(&dir);

        for i in 0..5 {
            // 每次启动前，当前日志写入上一批内容后轮转（与 sponsor 流程一致）
            write(&log, &format!("start {i} content\n"));
            rotate_service_log(&log, 3);
            // 轮转后当前文件应为空（尚未写入本次内容）
            assert_eq!(fs::read_to_string(&log).unwrap_or_default(), "");
            // 只允许保留 .0/.1/.2 三份
            assert!(!dir.join("dsh-web.log.3").exists());
            assert!(!dir.join("dsh-web.log.4").exists());
        }

        // 最后一次循环后：当前为空、.1 = start 4、.2 = start 3
        assert_eq!(fs::read_to_string(&log).unwrap_or_default(), "");
        assert!(fs::read_to_string(&dir.join("dsh-web.log.1"))
            .unwrap()
            .contains("start 4"));
        assert!(fs::read_to_string(&dir.join("dsh-web.log.2"))
            .unwrap()
            .contains("start 3"));
        assert!(!dir.join("dsh-web.log.3").exists());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn health_probe_plugin_urls_target_client_bundles_not_spa_root() {
        let urls = health_probe_plugin_urls(3080);
        assert!(urls.iter().all(|u| u.contains("/plugins/")));
        assert!(urls
            .iter()
            .all(|u| !u.ends_with("3080/") && !u.ends_with("://127.0.0.1:3080")));
        assert!(urls
            .iter()
            .any(|u| u.contains("dsh-client-ui-layout/client.js")));
    }

    #[test]
    fn authenticated_service_url_is_parsed_from_ready_line() {
        assert_eq!(
            parse_authenticated_service_url("dsh web: http://127.0.0.1:3081/?token=abc_DEF-123"),
            Some((3081, "http://127.0.0.1:3081/?token=abc_DEF-123".to_string()))
        );
        // LAN 地址即使跟在本机地址后，也只采纳第一个受限的回环地址。
        assert_eq!(
            parse_authenticated_service_url(
                "dsh web: http://127.0.0.1:3080/?token=local (LAN: http://10.0.0.2:3080/?token=lan)"
            )
            .map(|value| value.0),
            Some(3080)
        );
    }

    #[test]
    fn authenticated_service_url_rejects_untrusted_or_malformed_lines() {
        assert!(parse_authenticated_service_url("dsh web: http://127.0.0.1:3081").is_none());
        assert!(
            parse_authenticated_service_url("dsh web: http://example.com:3081/?token=secret")
                .is_none()
        );
        assert!(parse_authenticated_service_url(
            "dsh web: http://127.0.0.1:3081/path?token=secret"
        )
        .is_none());
        assert!(parse_authenticated_service_url(
            "dsh web: http://127.0.0.1:3081/?token=secret&next=evil"
        )
        .is_none());
    }

    #[test]
    fn authenticated_service_url_can_use_same_site_loopback_host() {
        let (port, raw) =
            parse_authenticated_service_url("dsh web: http://127.0.0.1:3081/?token=abc_DEF-123")
                .expect("parse authenticated URL");
        let mut url = reqwest::Url::parse(&raw).expect("parse stored URL");
        url.set_host(Some("dsh.tauri.localhost"))
            .expect("replace loopback host");
        assert_eq!(port, 3081);
        assert_eq!(
            url.as_str(),
            "http://dsh.tauri.localhost:3081/?token=abc_DEF-123"
        );
    }

    #[test]
    fn spa_html_fallback_is_not_a_plugin_bundle() {
        assert!(!looks_like_plugin_bundle(
            true,
            "<!doctype html><html lang=\"en\"><body>HARNESS Loading plugins...</body></html>"
        ));
        assert!(!looks_like_plugin_bundle(
            true,
            "<html><head></head></html>"
        ));
        assert!(!looks_like_plugin_bundle(true, "   "));
        assert!(!looks_like_plugin_bundle(
            false,
            "window.__ModuleLoader__={}"
        ));
        assert!(looks_like_plugin_bundle(
            true,
            "window.__ModuleLoader__.load({id:\"@deepseek-ai/dsh-client-ui-layout\"})"
        ));
    }

    /// keep=0 时把当前日志也删掉。
    #[test]
    fn rotate_with_keep_zero_removes_all() {
        let dir = std::env::temp_dir().join(format!("dsh_rotate_zero_{}", std::process::id()));
        let log = dir.join("dsh-web.log");
        let _ = fs::remove_dir_all(&dir);
        write(&log, "x");
        write(&dir.join("dsh-web.log.1"), "x");
        rotate_service_log(&log, 0);
        assert!(!log.exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
