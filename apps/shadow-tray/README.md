# shadow-tray

Menu-bar (macOS) and system-tray (Windows) companion for Shadow. It shows whether the
local proxy is protecting this machine, today's counts, and opens the activity dashboard
(`http://127.0.0.1:8877/shadow/ui`) in a small popover window.

Built with `tray-icon` + `tao` + `wry`: a native status item and the system webview
(WKWebView on macOS, WebView2 on Windows), a few MB in total, no bundled browser. It is a
separate Cargo workspace so the main workspace and Linux CI never need webview libraries.

Installed by `packaging/install.sh` on macOS; `shadow install --apply` starts it at login. To install it into an existing setup:

```sh
cargo install --locked --git https://github.com/Tiny-Bubble-Company/shadow shadow-tray --root ~/.local
shadow install --apply
```

From a checkout:

```sh
cd apps/shadow-tray
cargo run --release              # needs `shadow run` (or the service) for live numbers
cargo run --release -- --open    # open the dashboard popover immediately
cargo run --release -- --url http://127.0.0.1:9000
```

macOS: no Dock icon; left-click the crescent for the menu. Windows: left-click toggles the
dashboard popover, right-click opens the menu. Linux builds need GTK and WebKitGTK
development packages and are not tested.

The app only talks to the proxy on loopback and stores nothing.
