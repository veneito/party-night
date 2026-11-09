// Party game web server. TV opens /tv (picks a game, gets a room code), phones open / on the same Wi-Fi.
// /test shows a TV plus a few phones in one window; "Add bots" in the lobby fills the room for solo testing.
// Phones and the TV get live updates over server-sent events and send actions with plain POST requests.
use crate::room::{self, act, public_state, rooms};
use axum::extract::{ConnectInfo, Path, Query};
use axum::http::{header, StatusCode, Uri};
use axum::response::sse::{Event, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{extract::DefaultBodyLimit, Json, Router};
use base64::Engine;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::OnceLock;
use tokio::net::TcpListener;
use tokio_stream::{wrappers::UnboundedReceiverStream, StreamExt};

static PORT: AtomicU16 = AtomicU16::new(3000);
// set by the desktop app: handles /app/* (version, updates, fullscreen) for its own window only
pub static APP_CMD: OnceLock<fn(&str) -> String> = OnceLock::new();

// listens on the given port, or the next free one if it's taken
pub async fn bind(port: u16) -> std::io::Result<(u16, TcpListener)> {
    let mut last = None;
    for p in port..port + 20 {
        match TcpListener::bind(("0.0.0.0", p)).await {
            Ok(l) => { PORT.store(p, Ordering::Relaxed); return Ok((p, l)) }
            Err(e) => last = Some(e),
        }
    }
    Err(last.unwrap())
}

pub async fn run(listener: TcpListener) {
    tokio::spawn(async {
        let mut every = tokio::time::interval(std::time::Duration::from_millis(100));
        for n in 1u64.. { every.tick().await; room::tick(n) }
    });
    let app = Router::new()
        .route("/events", get(events))
        .route("/action", post(action))
        .route("/photo/{code}/{pid}", get(photo))
        .route("/ip", get(|| async { lan_url() }))
        .route("/qr.svg", get(qr))
        .route("/app/{cmd}", get(app_cmd))
        .fallback(page)
        .layer(DefaultBodyLimit::max(40_000));
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await.unwrap();
}

async fn events(Query(q): Query<HashMap<String, String>>) -> Response {
    let mut rooms = rooms();
    let Some(g) = rooms.get_mut(&q.get("room").map(|r| r.to_uppercase()).unwrap_or_default()) else { return StatusCode::NOT_FOUND.into_response() };
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let _ = tx.send(public_state(g).to_string());
    g.clients.push(tx);
    Sse::new(UnboundedReceiverStream::new(rx).map(|d| Ok::<_, Infallible>(Event::default().data(d)))).into_response()
}

async fn action(body: String) -> Response {
    let out = match serde_json::from_str::<Value>(&body) {
        // a bug in one game must not take the server down mid-party
        Ok(a) if a.is_object() => std::panic::catch_unwind(|| act(&a)).unwrap_or_else(|_| json!({ "error": "Bad request" })),
        _ => json!({ "error": "Bad request" }),
    };
    (if out.get("error").is_some() { StatusCode::BAD_REQUEST } else { StatusCode::OK }, Json(out)).into_response()
}

async fn photo(Path((code, pid)): Path<(String, String)>) -> Response {
    let rooms = rooms();
    let Some(data) = rooms.get(&code).and_then(|g| g.players.get(&pid)?.photo.as_ref()?.split_once(',').map(|x| x.1.to_string())) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(data) else { return StatusCode::NOT_FOUND.into_response() };
    // ?v= changes when the photo does
    ([(header::CONTENT_TYPE, "image/jpeg"), (header::CACHE_CONTROL, "max-age=86400")], bytes).into_response()
}

// scan-to-join: opens the phone page with the room code filled in
async fn qr(Query(q): Query<HashMap<String, String>>) -> Response {
    let room: String = q.get("room").map_or("", |r| r).chars().filter(|c| c.is_ascii_alphabetic()).take(4).collect::<String>().to_uppercase();
    let Ok(code) = qrcode::QrCode::new(format!("{}/?room={room}", lan_url())) else { return StatusCode::INTERNAL_SERVER_ERROR.into_response() };
    let w = code.width();
    let mut path = String::new();
    for (i, c) in code.to_colors().iter().enumerate() {
        if *c == qrcode::Color::Dark { path += &format!("M{} {}h1v1h-1z", i % w + 1, i / w + 1) }
    }
    let s = w + 2; // 1-module margin
    let svg = format!(r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {s} {s}" shape-rendering="crispEdges"><path fill="#ffffff" d="M0 0h{s}v{s}H0z"/><path fill="#1a1230" d="{path}"/></svg>"##);
    ([(header::CONTENT_TYPE, "image/svg+xml"), (header::CACHE_CONTROL, "no-store")], svg).into_response()
}

// only the app's own window (on this computer) may use these, never a phone on the Wi-Fi
async fn app_cmd(ConnectInfo(from): ConnectInfo<SocketAddr>, Path(cmd): Path<String>) -> Response {
    match APP_CMD.get() {
        Some(f) if from.ip().is_loopback() => f(&cmd).into_response(),
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn page(uri: Uri) -> Response {
    let p = uri.path();
    let (body, kind) = match p {
        "/style.css" => (include_str!("../../public/style.css"), "text/css"),
        "/common.js" => (include_str!("../../public/common.js"), "text/javascript"),
        _ if p.starts_with("/tv") => (include_str!("../../public/tv.html"), "text/html; charset=utf-8"),
        _ if p.starts_with("/test") => (include_str!("../../public/test.html"), "text/html; charset=utf-8"),
        _ => (include_str!("../../public/phone.html"), "text/html; charset=utf-8"),
    };
    ([(header::CONTENT_TYPE, kind)], body).into_response()
}

// The address phones should type. Skips virtual adapters (VirtualBox, Hyper-V, WSL, VPNs) that phones can't reach.
pub fn lan_url() -> String {
    const VIRTUAL: [&str; 11] = ["virtual", "vbox", "vmware", "vethernet", "wsl", "hyper-v", "docker", "tailscale", "zerotier", "hamachi", "loopback"];
    let nets: Vec<(String, std::net::IpAddr)> = local_ip_address::list_afinet_netifas().unwrap_or_default().into_iter()
        .filter(|(name, ip)| match ip {
            std::net::IpAddr::V4(v4) => !v4.is_loopback() && !v4.is_link_local() && !VIRTUAL.iter().any(|v| name.to_lowercase().contains(v)),
            _ => false,
        })
        .map(|(n, ip)| (n.to_lowercase(), ip)).collect();
    let best = nets.iter().find(|(n, _)| ["wi-fi", "wifi", "wlan", "wireless"].iter().any(|k| n.contains(k)) || n.starts_with("wl")) // Linux: wlan0, wlp2s0
        .or_else(|| nets.iter().find(|(n, _)| n.contains("ethernet") || n.starts_with("en") || n.starts_with("eth")))
        .or(nets.first());
    format!("http://{}:{}", best.map_or("localhost".into(), |b| b.1.to_string()), PORT.load(Ordering::Relaxed))
}
