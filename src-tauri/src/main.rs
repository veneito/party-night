// Party Night desktop app: runs the game server and shows the TV screen in its own window.
// Phones on the same Wi-Fi join at the address shown on screen.
// `party-night --server` runs just the server (open http://localhost:3000/tv or /test in a browser).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod games;
mod room;
mod server;

use serde_json::{json, Value};
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_updater::{Update, UpdaterExt};

static APP: OnceLock<AppHandle> = OnceLock::new();
static PENDING: Mutex<Option<Update>> = Mutex::new(None);

// Gives the TV page a tiny bridge to the app (same shape the page already uses), plus the TV keys:
// F11 or Escape toggles fullscreen, the way a TV app should; F5 reloads.
const BRIDGE: &str = r#"
window.partyApp = {
  version: () => fetch('/app/version').then(r => r.text()),
  check: () => fetch('/app/check'),
  download: () => fetch('/app/download'),
  onUpdate: f => { window.__partyUpd = f; },
  quit: () => fetch('/app/quit'),
};
addEventListener('keydown', e => {
  if (e.key === 'F11') { e.preventDefault(); fetch('/app/f11'); }
  else if (e.key === 'Escape') fetch('/app/esc');
  else if (e.key === 'F5') { e.preventDefault(); location.reload(); }
});
"#;

fn port_env() -> u16 { std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(3000) }

fn main() {
    if std::env::args().any(|a| a == "--server") {
        return tokio::runtime::Runtime::new().unwrap().block_on(async {
            let (p, l) = server::bind(port_env()).await.expect("couldn't start the game server");
            println!("TV:     http://localhost:{p}/tv\nPhones: {}\nTest:   http://localhost:{p}/test", server::lan_url());
            server::run(l).await
        });
    }
    tauri::Builder::default()
        // a second launch just focuses the first window instead of starting another server
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(w) = app.get_webview_window("main") { let _ = w.unminimize(); let _ = w.set_focus(); }
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let _ = APP.set(app.handle().clone());
            let _ = server::APP_CMD.set(app_cmd);
            let (port, listener) = tauri::async_runtime::block_on(server::bind(port_env()))
                .map_err(|e| format!("Couldn't start the game server: {e}"))?;
            tauri::async_runtime::spawn(server::run(listener));
            let win = WebviewWindowBuilder::new(app, "main", WebviewUrl::External(format!("http://localhost:{port}/tv").parse()?))
                .title("Party Night")
                .inner_size(1600.0, 900.0)
                .min_inner_size(960.0, 540.0)
                .maximized(true)
                .background_color(tauri::window::Color(14, 31, 26, 255))
                .initialization_script(BRIDGE);
            // sound effects and read-aloud jokes should play without a click first (keeps Tauri's default flags too)
            #[cfg(windows)]
            let win = win.additional_browser_args("--autoplay-policy=no-user-gesture-required --disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection");
            win.build()?;
            Ok(())
        })
        // closing the window ends the app, and the game server with it
        .run(tauri::generate_context!())
        .expect("error while running Party Night");
}

fn app_cmd(cmd: &str) -> String {
    let app = APP.get().unwrap();
    let win = app.get_webview_window("main");
    match (cmd, &win) {
        ("version", _) => return app.package_info().version.to_string(),
        ("f11", Some(w)) => { let _ = w.set_fullscreen(!w.is_fullscreen().unwrap_or(false)); }
        ("esc", Some(w)) => { let _ = w.set_fullscreen(false); }
        ("quit", _) => app.exit(0),
        ("check", _) => { tauri::async_runtime::spawn(check()); }
        ("download", _) => { tauri::async_runtime::spawn(download()); }
        _ => {}
    }
    String::new()
}

// Updates come from GitHub releases. The app asks first (popup on the TV screen); on "Update" it downloads,
// installs and reopens itself.
fn tell(u: Value) {
    if let Some(w) = APP.get().and_then(|a| a.get_webview_window("main")) { let _ = w.eval(format!("window.__partyUpd && window.__partyUpd({u})")); }
}
fn err_line(e: impl std::fmt::Display) -> Value { json!({ "state": "error", "message": e.to_string().lines().next().unwrap_or("").to_string() }) }
async fn check() {
    if cfg!(debug_assertions) { return tell(json!({ "state": "error", "message": "updates only work in the installed app" })) }
    let found = match APP.get().unwrap().updater() { Ok(u) => u.check().await, Err(e) => return tell(err_line(e)) };
    match found {
        Ok(Some(u)) => { tell(json!({ "state": "available", "version": u.version })); *PENDING.lock().unwrap() = Some(u); }
        Ok(None) => tell(json!({ "state": "none" })),
        // the newest release has no latest.json (e.g. one from before the Tauri switch): nothing to update to
        Err(tauri_plugin_updater::Error::ReleaseNotFound) => tell(json!({ "state": "none" })),
        Err(e) => tell(err_line(e)),
    }
}
async fn download() {
    let Some(u) = PENDING.lock().unwrap().take() else { return };
    let (mut got, mut shown) = (0u64, -1i64);
    let bytes = u.download(|chunk, total| {
        got += chunk as u64;
        let pct = total.map_or(0, |t| (got * 100 / t.max(1)) as i64);
        if pct != shown { shown = pct; tell(json!({ "state": "progress", "percent": pct })) }
    }, || {}).await;
    match bytes {
        Ok(b) => {
            tell(json!({ "state": "installing" }));
            tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
            // on Windows the installer takes over and closes the app; restart covers the other platforms
            match u.install(b) { Ok(()) => APP.get().unwrap().restart(), Err(e) => tell(err_line(e)) }
        }
        Err(e) => tell(err_line(e)),
    }
}
