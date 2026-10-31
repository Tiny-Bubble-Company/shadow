//! Shadow menu-bar (macOS) / system-tray (Windows) app.
//!
//! Shows whether the local Shadow proxy is protecting this machine, today's counts, and a
//! popover with the activity dashboard (`/shadow/ui?embed=1`) in a native webview. It only
//! ever talks to the proxy on loopback; it holds no data of its own.
//!
//! Options: `--url http://127.0.0.1:8877` (or `SHADOW_URL`) to point at another port;
//! `--open` shows the dashboard popover right away.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod icon;

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use tao::dpi::{LogicalSize, PhysicalPosition};
use tao::event::{Event, StartCause, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use tao::window::{Window, WindowBuilder};
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{MouseButton, MouseButtonState, Rect, TrayIcon, TrayIconBuilder, TrayIconEvent};
use wry::{WebView, WebViewBuilder};

const DOCS_URL: &str = "https://tiny-bubble-company.github.io/shadow/concepts/activity-dashboard/";
const POPOVER: (f64, f64) = (420.0, 640.0);
const POLL: Duration = Duration::from_secs(5);

/// What the poller learned about the proxy.
#[derive(Clone, Debug, Default, PartialEq)]
struct Status {
    online: bool,
    requests: u64,
    values: u64,
    restored: u64,
    blocked: u64,
}

enum UserEvent {
    Tray(TrayIconEvent),
    Menu(MenuEvent),
    Status(Status),
}

fn base_url() -> String {
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--url" {
            if let Some(u) = args.next() {
                return u.trim_end_matches('/').to_string();
            }
        } else if let Some(u) = a.strip_prefix("--url=") {
            return u.trim_end_matches('/').to_string();
        }
    }
    std::env::var("SHADOW_URL")
        .ok()
        .filter(|u| !u.is_empty())
        .unwrap_or_else(|| "http://127.0.0.1:8877".into())
        .trim_end_matches('/')
        .to_string()
}

/// `host:port` of an `http://host:port` URL.
fn authority(url: &str) -> String {
    url.trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or("127.0.0.1:8877")
        .to_string()
}

/// Minimal HTTP/1.1 GET to the local proxy (plain HTTP on loopback; no extra dependencies).
fn http_get(addr: &str, path: &str) -> Option<String> {
    let sock = addr.to_socket_addrs().ok()?.next()?;
    let mut s = TcpStream::connect_timeout(&sock, Duration::from_millis(500)).ok()?;
    s.set_read_timeout(Some(Duration::from_secs(3))).ok()?;
    write!(
        s,
        "GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n"
    )
    .ok()?;
    let mut buf = Vec::new();
    s.read_to_end(&mut buf).ok()?;
    let text = String::from_utf8_lossy(&buf).into_owned();
    let (head, body) = text.split_once("\r\n\r\n")?;
    head.starts_with("HTTP/1.1 200").then(|| body.to_string())
}

fn poll(addr: &str) -> Status {
    let Some(body) = http_get(addr, "/shadow/activity/summary?window=today") else {
        // Old proxies without the activity API still count as "up".
        return Status {
            online: http_get(addr, "/healthz").is_some(),
            ..Status::default()
        };
    };
    let v: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    let n = |k: &str| v["totals"][k].as_u64().unwrap_or(0);
    Status {
        online: true,
        requests: n("requests"),
        values: n("values_protected"),
        restored: n("values_restored"),
        blocked: n("blocked"),
    }
}

fn spawn_poller(addr: String, proxy: EventLoopProxy<UserEvent>) {
    std::thread::spawn(move || loop {
        if proxy.send_event(UserEvent::Status(poll(&addr))).is_err() {
            return;
        }
        std::thread::sleep(POLL);
    });
}

fn open_external(url: &str) {
    let r = if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(url).spawn()
    } else if cfg!(windows) {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
    } else {
        std::process::Command::new("xdg-open").arg(url).spawn()
    };
    if let Err(e) = r {
        eprintln!("could not open {url}: {e}");
    }
}

fn fmt_n(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

struct Items {
    status: MenuItem,
    requests: MenuItem,
    values: MenuItem,
    blocked: MenuItem,
    dashboard: MenuItem,
    browser: MenuItem,
    docs: MenuItem,
    quit: MenuItem,
}

impl Items {
    fn new() -> Self {
        Self {
            status: MenuItem::new("Checking Shadow…", false, None),
            requests: MenuItem::new("Today: no requests yet", false, None),
            values: MenuItem::new("No values protected yet", false, None),
            blocked: MenuItem::new("Nothing blocked", false, None),
            dashboard: MenuItem::new("Open dashboard", true, None),
            browser: MenuItem::new("Open in browser", true, None),
            docs: MenuItem::new("Docs", true, None),
            quit: MenuItem::new("Quit Shadow menu", true, None),
        }
    }

    fn menu(&self) -> Menu {
        let menu = Menu::new();
        let sep = PredefinedMenuItem::separator;
        let _ = menu.append_items(&[
            &self.status,
            &sep(),
            &self.requests,
            &self.values,
            &self.blocked,
            &sep(),
            &self.dashboard,
            &self.browser,
            &self.docs,
            &sep(),
            &self.quit,
        ]);
        menu
    }

    fn update(&self, s: &Status) {
        let machine = if cfg!(target_os = "macos") {
            "this Mac"
        } else {
            "this PC"
        };
        if s.online {
            self.status
                .set_text(format!("● Shadow is protecting {machine}"));
            self.requests.set_text(format!(
                "Today: {} request{} monitored",
                fmt_n(s.requests),
                if s.requests == 1 { "" } else { "s" }
            ));
            self.values.set_text(format!(
                "{} values protected · {} restored",
                fmt_n(s.values),
                fmt_n(s.restored)
            ));
            self.blocked.set_text(if s.blocked == 0 {
                "Nothing blocked".to_string()
            } else {
                format!(
                    "{} request{} blocked",
                    fmt_n(s.blocked),
                    if s.blocked == 1 { "" } else { "s" }
                )
            });
        } else {
            self.status.set_text("○ Shadow is not running");
            self.requests.set_text("Start it with `shadow run`");
            self.values.set_text("or `shadow install --apply`");
            self.blocked.set_text("Your AI traffic is not protected");
        }
    }
}

fn offline_html() -> String {
    r#"<!doctype html><html><head><meta charset="utf-8"><meta name="color-scheme" content="light dark">
<style>
body{margin:0;height:100vh;display:grid;place-items:center;font:14px -apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;
background:#f6f6fb;color:#15142a;text-align:center}
@media (prefers-color-scheme:dark){body{background:#0c0b14;color:#eeecfb}}
.c{max-width:300px;padding:24px}.dot{width:56px;height:56px;border-radius:50%;margin:0 auto 18px;
background:radial-gradient(circle at 70% 30%,transparent 46%,#7c6cff 47%);opacity:.9}
h1{font-size:18px;margin:0 0 8px}p{color:#8a88a3;margin:0 0 14px;line-height:1.5}
code{background:rgba(124,108,255,.14);color:#7c6cff;padding:3px 8px;border-radius:6px;font:13px ui-monospace,Menlo,monospace}
</style></head><body><div class="c"><div class="dot"></div><h1>Shadow isn't running</h1>
<p>Your AI traffic is not being protected right now. Start Shadow and this view will connect on its own.</p>
<code>shadow run</code></div></body></html>"#
        .to_string()
}

struct Popover {
    window: Window,
    webview: WebView,
    online: bool,
}

fn build_popover(
    target: &tao::event_loop::EventLoopWindowTarget<UserEvent>,
    url: &str,
    online: bool,
) -> Result<Popover, Box<dyn std::error::Error>> {
    let builder = WindowBuilder::new()
        .with_title("Shadow")
        .with_inner_size(LogicalSize::new(POPOVER.0, POPOVER.1))
        .with_min_inner_size(LogicalSize::new(360.0, 480.0))
        .with_visible(false)
        .with_decorations(false)
        .with_always_on_top(true)
        .with_minimizable(false)
        .with_maximizable(false);
    #[cfg(windows)]
    let builder = {
        use tao::platform::windows::WindowBuilderExtWindows;
        builder.with_skip_taskbar(true)
    };
    let window = builder.build(target)?;
    let wb = if online {
        WebViewBuilder::new().with_url(url)
    } else {
        WebViewBuilder::new().with_html(offline_html())
    };
    let wb = wb
        .with_devtools(cfg!(debug_assertions))
        .with_navigation_handler(|u| {
            // Stay on the local dashboard; anything else opens in the real browser.
            let local = u.starts_with("http://127.0.0.1")
                || u.starts_with("http://localhost")
                || u.starts_with("http://[::1]")
                || u.starts_with("about:")
                || u.starts_with("data:");
            if !local {
                open_external(&u);
            }
            local
        });
    #[cfg(not(target_os = "linux"))]
    let webview = wb.build(&window)?;
    #[cfg(target_os = "linux")]
    let webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        wb.build_gtk(window.default_vbox().ok_or("no GTK vbox")?)?
    };
    Ok(Popover {
        window,
        webview,
        online,
    })
}

/// Place the popover under (macOS) or above (Windows taskbar) the tray icon.
fn position(window: &Window, anchor: Option<Rect>) {
    let size = window.outer_size();
    let monitor = window
        .current_monitor()
        .or_else(|| window.primary_monitor());
    let (mx, my, mw, mh) = monitor
        .map(|m| {
            let p = m.position();
            let s = m.size();
            (p.x as f64, p.y as f64, s.width as f64, s.height as f64)
        })
        .unwrap_or((0.0, 0.0, 1440.0, 900.0));
    let (w, h) = (size.width as f64, size.height as f64);
    let (mut x, mut y) = match anchor {
        Some(r) => {
            let cx = r.position.x + r.size.width as f64 / 2.0;
            let below = r.position.y + r.size.height as f64 + 6.0;
            let above = r.position.y - h - 6.0;
            // Tray at the top of the screen (macOS menu bar) -> drop down; else pop up.
            let y = if r.position.y < my + mh / 2.0 {
                below
            } else {
                above
            };
            (cx - w / 2.0, y)
        }
        None => (mx + mw - w - 16.0, my + 36.0),
    };
    x = x.clamp(mx + 8.0, (mx + mw - w - 8.0).max(mx));
    y = y.clamp(my + 8.0, (my + mh - h - 8.0).max(my));
    window.set_outer_position(PhysicalPosition::new(x, y));
}

fn main() {
    let base = base_url();
    let open_now = std::env::args().any(|a| a == "--open");
    let dashboard = format!("{base}/shadow/ui?embed=1&w=24h");
    let browser_url = format!("{base}/shadow/ui");

    #[allow(unused_mut)]
    let mut event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    #[cfg(target_os = "macos")]
    {
        use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
        // Menu-bar only: no Dock icon, no app menu.
        event_loop.set_activation_policy(ActivationPolicy::Accessory);
    }
    let proxy = event_loop.create_proxy();
    {
        let p = proxy.clone();
        TrayIconEvent::set_event_handler(Some(move |e| {
            let _ = p.send_event(UserEvent::Tray(e));
        }));
        let p = proxy.clone();
        MenuEvent::set_event_handler(Some(move |e| {
            let _ = p.send_event(UserEvent::Menu(e));
        }));
    }
    spawn_poller(authority(&base), proxy);

    let items = Items::new();
    let mut tray: Option<TrayIcon> = None;
    let mut popover: Option<Popover> = None;
    let mut anchor: Option<Rect> = None;
    let mut status = Status::default();
    let ids: [MenuId; 4] = [
        items.dashboard.id().clone(),
        items.browser.id().clone(),
        items.docs.id().clone(),
        items.quit.id().clone(),
    ];

    event_loop.run(move |event, target, control_flow| {
        *control_flow = ControlFlow::Wait;
        let show = |popover: &mut Option<Popover>, anchor: Option<Rect>, online: bool| {
            if popover.is_none() {
                match build_popover(target, &dashboard, online) {
                    Ok(p) => *popover = Some(p),
                    Err(e) => {
                        eprintln!("could not open the dashboard window: {e}");
                        open_external(&browser_url);
                        return;
                    }
                }
            }
            if let Some(p) = popover {
                position(&p.window, anchor);
                p.window.set_visible(true);
                p.window.set_focus();
            }
        };
        match event {
            Event::NewEvents(StartCause::Init) => {
                // macOS requires the status item to be created once the loop is running.
                let built = TrayIconBuilder::new()
                    .with_menu(Box::new(items.menu()))
                    .with_tooltip("Shadow")
                    .with_icon(icon::tray_icon(false))
                    .with_icon_as_template(cfg!(target_os = "macos"))
                    .with_menu_on_left_click(cfg!(target_os = "macos"))
                    .build();
                match built {
                    Ok(t) => {
                        tray = Some(t);
                        if open_now {
                            let online = poll(&authority(&base)).online;
                            show(&mut popover, None, online);
                        }
                    }
                    Err(e) => {
                        eprintln!("could not create the tray icon: {e}");
                        *control_flow = ControlFlow::Exit;
                    }
                }
            }
            Event::UserEvent(UserEvent::Status(s)) => {
                if s != status {
                    let was_online = status.online;
                    items.update(&s);
                    if let Some(t) = &tray {
                        if s.online != was_online || status == Status::default() {
                            let _ = t.set_icon(Some(icon::tray_icon(s.online)));
                            t.set_icon_as_template(cfg!(target_os = "macos"));
                        }
                        let tip = if s.online {
                            format!(
                                "Shadow · protecting · {} values protected today",
                                fmt_n(s.values)
                            )
                        } else {
                            "Shadow · not running".to_string()
                        };
                        let _ = t.set_tooltip(Some(tip));
                    }
                    if let Some(p) = popover.as_mut() {
                        if s.online != p.online {
                            p.online = s.online;
                            let _ = if s.online {
                                p.webview.load_url(&dashboard)
                            } else {
                                p.webview.load_html(&offline_html())
                            };
                        }
                    }
                    status = s;
                }
            }
            Event::UserEvent(UserEvent::Tray(e)) => match e {
                TrayIconEvent::Click {
                    rect,
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } => {
                    anchor = Some(rect);
                    // Windows / Linux: left click toggles the popover (right click = menu).
                    if !cfg!(target_os = "macos") {
                        let visible = popover.as_ref().is_some_and(|p| p.window.is_visible());
                        if visible {
                            if let Some(p) = &popover {
                                p.window.set_visible(false);
                            }
                        } else {
                            show(&mut popover, anchor, status.online);
                        }
                    }
                }
                TrayIconEvent::Click { rect, .. }
                | TrayIconEvent::Enter { rect, .. }
                | TrayIconEvent::Move { rect, .. } => anchor = Some(rect),
                _ => {}
            },
            Event::UserEvent(UserEvent::Menu(e)) => {
                if e.id == ids[0] {
                    show(&mut popover, anchor, status.online);
                } else if e.id == ids[1] {
                    open_external(&browser_url);
                } else if e.id == ids[2] {
                    open_external(DOCS_URL);
                } else if e.id == ids[3] {
                    tray.take();
                    *control_flow = ControlFlow::Exit;
                }
            }
            // Behave like a popover: hide when it loses focus or is closed.
            Event::WindowEvent {
                event: WindowEvent::Focused(false) | WindowEvent::CloseRequested,
                ..
            } => {
                if let Some(p) = &popover {
                    p.window.set_visible(false);
                }
            }
            _ => {}
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helpers() {
        assert_eq!(authority("http://127.0.0.1:8877"), "127.0.0.1:8877");
        assert_eq!(authority("http://localhost:9000/x"), "localhost:9000");
        assert_eq!(fmt_n(1234567), "1,234,567");
        let s = poll("127.0.0.1:9");
        assert!(!s.online);
    }

    #[test]
    fn icons_render() {
        for on in [true, false] {
            let (rgba, w, h) = icon::rgba(on, false);
            assert_eq!(rgba.len(), (w * h * 4) as usize);
            assert!(rgba.chunks(4).any(|p| p[3] > 0));
        }
    }
}
