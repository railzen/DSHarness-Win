//! 官方 dsh boot 页卡在 “Loading plugins…” 时的一次性自动刷新。
//!
//! 桌面端曾把 SPA `/` 的 200 当成就绪并立刻挂 iframe。连接桥尚未注册时，
//! boot 页会无限转圈（[#36](https://github.com/dsh-tauri-desk/deepseek-harness-desktop/issues/36)、
//! [#42](https://github.com/dsh-tauri-desk/deepseek-harness-desktop/issues/42)）；
//! 手动刷新即可恢复。本脚本注入 iframe，若仍停在 boot 文案则只 reload 一次。
//!
//! 注入通道与 [`crate::desktop::nav::NAV_SHIM_JS`] 相同。

/// Windows 桌面壳持有并只向回环地址启动 dsh Host。认证 iframe 必须使用
/// `dsh.tauri.localhost` 保持 SameSite Cookie，因此在插件启动前通过 dsh 官方
/// transport hook 明确声明 Host 归当前壳所有，让本机设置仍使用持久化 Host scope。
#[cfg(windows)]
pub(crate) const EMBEDDED_HOST_TRANSPORT_JS: &str = r#"(function () {
  if (location.hostname !== 'dsh.tauri.localhost') return;
  var current = globalThis.__DSH_TRANSPORT__;
  if (current != null && typeof current !== 'object') return;
  globalThis.__DSH_TRANSPORT__ = Object.assign({}, current || {}, { ownsHost: true });
})();"#;

/// iframe 内：boot 页卡住时自动 reload 一次（sessionStorage 防循环）。
pub(crate) const PLUGIN_BOOT_RELOAD_JS: &str = r#"(function () {
  if (window.__dsh_plugin_boot_reload__) return;
  window.__dsh_plugin_boot_reload__ = true;
  if (window === window.top) return;

  var FLAG = '__dsh_plugin_boot_reloaded__';
  try {
    if (sessionStorage.getItem(FLAG) === '1') return;
  } catch (_) {}

  function isSplash() {
    var text = (document.body && (document.body.innerText || document.body.textContent)) || '';
    return text.indexOf('Loading plugins') !== -1;
  }

  function tick() {
    if (!isSplash()) return;
    try { sessionStorage.setItem(FLAG, '1'); } catch (_) {}
    location.reload();
  }

  setTimeout(tick, 8000);
})();"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boot_reload_script_is_one_shot_and_splash_scoped() {
        assert!(PLUGIN_BOOT_RELOAD_JS.contains("__dsh_plugin_boot_reloaded__"));
        assert!(PLUGIN_BOOT_RELOAD_JS.contains("Loading plugins"));
        assert!(PLUGIN_BOOT_RELOAD_JS.contains("sessionStorage"));
        assert!(PLUGIN_BOOT_RELOAD_JS.contains("window === window.top"));
        assert!(PLUGIN_BOOT_RELOAD_JS.contains("location.reload"));
    }

    #[cfg(windows)]
    #[test]
    fn embedded_host_transport_is_scoped_and_marks_host_owned() {
        assert!(EMBEDDED_HOST_TRANSPORT_JS.contains("dsh.tauri.localhost"));
        assert!(EMBEDDED_HOST_TRANSPORT_JS.contains("__DSH_TRANSPORT__"));
        assert!(EMBEDDED_HOST_TRANSPORT_JS.contains("ownsHost: true"));
    }
}
