//! `shadow top`: a live terminal dashboard of what Shadow is doing.

use std::time::{Duration, Instant};

use crossterm::event::{self, Event as TermEvent, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Bar, BarChart, BarGroup, Block, BorderType, Cell, Clear, Padding, Paragraph, Row, Table,
    TableState, Tabs, Wrap,
};
use ratatui::{DefaultTerminal, Frame};
use shadow_core::activity::{provider_label, Action, Breakdown, Event, Window};

use crate::activity::{describe_kinds, fmt_n, pct, Feed, Snapshot, Source};

const ACCENT: Color = Color::Rgb(124, 108, 255);
const ACCENT_SOFT: Color = Color::Rgb(169, 157, 255);
const DANGER: Color = Color::Rgb(239, 79, 107);
const OK: Color = Color::Rgb(31, 174, 138);
const MUTED: Color = Color::Rgb(140, 138, 165);
const WARN: Color = Color::Rgb(245, 165, 36);

const TABS: [&str; 3] = ["Overview", "Incidents", "Breakdown"];

struct App {
    source: Source,
    window: Window,
    tab: usize,
    snap: Snapshot,
    table: TableState,
    detail: bool,
    last: Instant,
}

impl App {
    fn refresh(&mut self) {
        self.snap = self.source.fetch(self.window, 200);
        self.last = Instant::now();
        let n = self.snap.incidents.len();
        match self.table.selected() {
            Some(i) if i >= n => self.table.select(n.checked_sub(1)),
            None if n > 0 => self.table.select(Some(0)),
            _ => {}
        }
    }
}

pub fn run(source: Source) -> anyhow::Result<()> {
    let window = Window::Day;
    let snap = source.fetch(window, 200);
    let mut app = App {
        source,
        window,
        tab: 0,
        snap,
        table: TableState::default().with_selected(Some(0)),
        detail: false,
        last: Instant::now(),
    };
    let mut terminal = ratatui::init();
    let res = event_loop(&mut terminal, &mut app);
    ratatui::restore();
    res
}

fn event_loop(terminal: &mut DefaultTerminal, app: &mut App) -> anyhow::Result<()> {
    let every = if matches!(app.snap.state, Feed::Demo) {
        Duration::from_secs(30)
    } else {
        Duration::from_secs(2)
    };
    loop {
        terminal.draw(|f| draw(f, app))?;
        let wait = every.saturating_sub(app.last.elapsed());
        if event::poll(wait)? {
            let TermEvent::Key(k) = event::read()? else {
                continue;
            };
            if k.kind != KeyEventKind::Press {
                continue;
            }
            let n = app.snap.incidents.len();
            match k.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => return Ok(()),
                KeyCode::Esc if app.detail => app.detail = false,
                KeyCode::Esc => return Ok(()),
                KeyCode::Char(c @ '1'..='4') => {
                    app.window = Window::ALL[(c as u8 - b'1') as usize];
                    app.refresh();
                }
                KeyCode::Char('w') => {
                    let i = Window::ALL
                        .iter()
                        .position(|w| *w == app.window)
                        .unwrap_or(0);
                    app.window = Window::ALL[(i + 1) % Window::ALL.len()];
                    app.refresh();
                }
                KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
                    app.tab = (app.tab + 1) % TABS.len();
                    app.detail = false;
                }
                KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
                    app.tab = (app.tab + TABS.len() - 1) % TABS.len();
                    app.detail = false;
                }
                KeyCode::Char('o') => app.tab = 0,
                KeyCode::Char('i') => app.tab = 1,
                KeyCode::Char('b') => app.tab = 2,
                KeyCode::Down | KeyCode::Char('j') if n > 0 => {
                    let i = app.table.selected().map_or(0, |i| (i + 1).min(n - 1));
                    app.table.select(Some(i));
                }
                KeyCode::Up | KeyCode::Char('k') if n > 0 => {
                    let i = app.table.selected().map_or(0, |i| i.saturating_sub(1));
                    app.table.select(Some(i));
                }
                KeyCode::Enter if n > 0 => {
                    app.tab = 1;
                    app.detail = !app.detail;
                }
                KeyCode::Char('r') => app.refresh(),
                _ => {}
            }
        } else {
            app.refresh();
        }
    }
}

fn card(title: &str) -> Block<'_> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(Color::Rgb(70, 66, 100)))
        .title(Line::from(format!(" {title} ")).bold())
        .padding(Padding::horizontal(1))
}

fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let [head, tabs, body, foot] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Min(8),
        Constraint::Length(1),
    ])
    .areas(area);
    draw_header(f, head, app);
    let titles: Vec<Line> = TABS
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let n = if i == 1 {
                format!(" {t} ({}) ", app.snap.incidents.len().min(200))
            } else {
                format!(" {t} ")
            };
            Line::from(n)
        })
        .collect();
    f.render_widget(
        Tabs::new(titles)
            .select(app.tab)
            .style(Style::new().fg(MUTED))
            .highlight_style(Style::new().fg(Color::White).bg(ACCENT).bold())
            .divider(" ")
            .padding("", ""),
        tabs.inner(ratatui::layout::Margin::new(1, 0)),
    );
    match app.tab {
        0 => draw_overview(f, body, app),
        1 => draw_incidents(f, body, app),
        _ => draw_breakdown(f, body, app),
    }
    let keys = Line::from(vec![
        " q ".fg(Color::Black).bg(MUTED),
        " quit  ".fg(MUTED),
        " 1-4 ".fg(Color::Black).bg(MUTED),
        " 1h/24h/7d/30d  ".fg(MUTED),
        " tab ".fg(Color::Black).bg(MUTED),
        " switch view  ".fg(MUTED),
        " ↑↓ ⏎ ".fg(Color::Black).bg(MUTED),
        " incidents  ".fg(MUTED),
        " r ".fg(Color::Black).bg(MUTED),
        " refresh".fg(MUTED),
    ]);
    f.render_widget(Paragraph::new(keys), foot);
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.snap.summary.totals;
    let status = match &app.snap.state {
        Feed::Online => Span::styled(" ● Protected ", Style::new().fg(Color::Black).bg(OK).bold()),
        Feed::Offline(_) => Span::styled(
            " ○ Proxy offline ",
            Style::new().fg(Color::White).bg(DANGER).bold(),
        ),
        Feed::Demo => Span::styled(" DEMO DATA ", Style::new().fg(Color::Black).bg(WARN).bold()),
    };
    let mut wins = vec![];
    for (i, w) in Window::ALL.iter().enumerate() {
        let s = format!(" {} {} ", i + 1, w.as_str());
        wins.push(if *w == app.window {
            Span::styled(s, Style::new().fg(Color::White).bg(ACCENT).bold())
        } else {
            Span::styled(s, Style::new().fg(MUTED))
        });
    }
    let mut l1 = vec![
        Span::styled(" ◐ Shadow ", Style::new().fg(ACCENT).bold()),
        Span::styled("privacy activity  ", Style::new().fg(MUTED)),
        status,
        Span::raw("   "),
    ];
    l1.extend(wins);
    let l2 = match &app.snap.state {
        Feed::Offline(_) => Line::from(vec![
            Span::styled(
                format!(" Shadow isn't answering on {}. ", app.source.addr()),
                Style::new().fg(DANGER),
            ),
            Span::styled(
                "Start it with `shadow run`; showing the saved log meanwhile. Retrying…",
                Style::new().fg(MUTED),
            ),
        ]),
        _ => Line::from(vec![
            Span::styled(format!(" {} ", fmt_n(t.requests)), Style::new().bold()),
            Span::styled("requests  ", Style::new().fg(MUTED)),
            Span::styled(
                format!("{} ", fmt_n(t.values_protected)),
                Style::new().fg(ACCENT).bold(),
            ),
            Span::styled("values protected  ", Style::new().fg(MUTED)),
            Span::styled(
                format!("{} ", fmt_n(t.values_restored)),
                Style::new().fg(OK).bold(),
            ),
            Span::styled("restored  ", Style::new().fg(MUTED)),
            Span::styled(
                format!("{} ", fmt_n(t.blocked)),
                Style::new().fg(DANGER).bold(),
            ),
            Span::styled("blocked  ", Style::new().fg(MUTED)),
            Span::styled(format!("{} ", t.providers), Style::new().bold()),
            Span::styled("destinations  ", Style::new().fg(MUTED)),
            Span::styled(format!("{} ", t.clients), Style::new().bold()),
            Span::styled("apps", Style::new().fg(MUTED)),
        ]),
    };
    f.render_widget(Paragraph::new(vec![Line::from(l1), l2]), area);
}

fn draw_overview(f: &mut Frame, area: Rect, app: &App) {
    let s = &app.snap.summary;
    let t = &s.totals;
    let [stats, timeline, lower] = Layout::vertical([
        Constraint::Length(4),
        Constraint::Percentage(45),
        Constraint::Min(6),
    ])
    .areas(area);
    let cols: [Rect; 4] = Layout::horizontal([Constraint::Ratio(1, 4); 4]).areas(stats);
    let stat = |label: &'static str, value: u64, sub: String, color: Color| {
        Paragraph::new(vec![
            Line::from(Span::styled(
                fmt_n(value),
                Style::new().fg(color).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(sub, Style::new().fg(MUTED))),
        ])
        .block(card(label))
    };
    f.render_widget(
        stat(
            "Requests monitored",
            t.requests,
            format!("{} had sensitive data", pct(t.masked_requests, t.requests)),
            Color::White,
        ),
        cols[0],
    );
    f.render_widget(
        stat(
            "Values protected",
            t.values_protected,
            format!("{} restored", fmt_n(t.values_restored)),
            ACCENT,
        ),
        cols[1],
    );
    f.render_widget(
        stat(
            "Requests blocked",
            t.blocked,
            s.block_reasons
                .first()
                .map(|b| b.label.clone())
                .unwrap_or_else(|| "nothing blocked".into()),
            DANGER,
        ),
        cols[2],
    );
    f.render_widget(
        stat(
            "AI destinations",
            t.providers,
            s.providers
                .first()
                .map(|p| format!("mostly {}", provider_label(&p.key)))
                .unwrap_or_else(|| "none yet".into()),
            OK,
        ),
        cols[3],
    );

    // Timeline: requests as bars, protected values as a sparkline underneath.
    let block = card("Traffic").title_bottom(
        Line::from(vec![
            Span::styled(" ▇ ", Style::new().fg(ACCENT)),
            Span::styled("requests  ", Style::new().fg(MUTED)),
            Span::styled("▇ ", Style::new().fg(DANGER)),
            Span::styled("blocked  ", Style::new().fg(MUTED)),
            Span::styled("▁▃▅ ", Style::new().fg(ACCENT_SOFT)),
            Span::styled("values protected ", Style::new().fg(MUTED)),
        ])
        .right_aligned(),
    );
    let inner = block.inner(timeline);
    f.render_widget(block, timeline);
    let tl = &s.timeline;
    let width = inner.width.max(1) as usize;
    let gap = if tl.len() * 2 <= width { 1 } else { 0 };
    let bw = ((width + gap) / tl.len().max(1)).saturating_sub(gap).max(1);
    // Centre the chart in the leftover columns.
    let used = (tl.len() * (bw + gap)).saturating_sub(gap).min(width) as u16;
    let inner = Rect {
        x: inner.x + (inner.width - used) / 2,
        width: used.max(1),
        ..inner
    };
    let [bars_area, blocked_area, spark_area] = Layout::vertical([
        Constraint::Min(3),
        Constraint::Length(1),
        Constraint::Length(2),
    ])
    .areas(inner);
    // Requests per bucket; blocked requests get their own row underneath, so one block
    // doesn't paint a whole busy bucket red.
    let chart = |bars, area: Rect, f: &mut Frame| {
        f.render_widget(
            BarChart::default()
                .data(BarGroup::default().bars(bars))
                .bar_width(bw as u16)
                .bar_gap(gap as u16),
            area,
        );
    };
    let requests: Vec<Bar> = tl.iter().map(|p| bar(p.requests, ACCENT)).collect();
    chart(&requests, bars_area, f);
    let blocked: Vec<Bar> = tl.iter().map(|p| bar(p.blocked.min(1), DANGER)).collect();
    chart(&blocked, blocked_area, f);
    // Same geometry as the bars above so both series line up bucket by bucket.
    let values: Vec<Bar> = tl.iter().map(|p| bar(p.values, ACCENT_SOFT)).collect();
    chart(&values, spark_area, f);

    let [a, b, c] = Layout::horizontal([
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
    ])
    .areas(lower);
    bar_list(f, a, "What was protected", &s.kinds, |r| r.values, ACCENT);
    let providers: Vec<Breakdown> = s
        .providers
        .iter()
        .map(|p| Breakdown {
            label: provider_label(&p.key),
            ..p.clone()
        })
        .collect();
    bar_list(f, b, "Where it was going", &providers, |r| r.requests, OK);
    bar_list(
        f,
        c,
        "Which apps sent it",
        &s.clients,
        |r| r.requests,
        ACCENT_SOFT,
    );
}

/// Horizontal bars with eighth-block precision.
fn bar_list(
    f: &mut Frame,
    area: Rect,
    title: &str,
    rows: &[Breakdown],
    value: impl Fn(&Breakdown) -> u64,
    color: Color,
) {
    let block = card(title);
    let inner = block.inner(area);
    f.render_widget(block, area);
    if rows.is_empty() {
        f.render_widget(
            Paragraph::new("nothing yet in this window").fg(MUTED),
            inner,
        );
        return;
    }
    let max = rows.iter().map(&value).max().unwrap_or(1).max(1);
    let w = inner.width as usize;
    let label_w = (w * 2 / 5).clamp(8, 22);
    let num_w = 7;
    let bar_w = w.saturating_sub(label_w + num_w + 2).max(1);
    let lines: Vec<Line> = rows
        .iter()
        .take(inner.height as usize)
        .map(|r| {
            let v = value(r);
            let eighths = (v as f64 / max as f64 * bar_w as f64 * 8.0).round() as usize;
            let mut bar = "█".repeat(eighths / 8);
            if eighths % 8 > 0 {
                bar.push(['▏', '▎', '▍', '▌', '▋', '▊', '▉'][eighths % 8 - 1]);
            }
            if bar.is_empty() {
                bar.push('▏');
            }
            let mut label: String = r.label.chars().take(label_w).collect();
            if r.label.chars().count() > label_w {
                label.pop();
                label.push('…');
            }
            Line::from(vec![
                Span::raw(format!("{label:<label_w$} ")),
                Span::styled(format!("{bar:<bar_w$}"), Style::new().fg(color)),
                Span::styled(format!(" {:>num_w$}", fmt_n(v)), Style::new().bold()),
            ])
        })
        .collect();
    f.render_widget(Paragraph::new(lines), inner);
}

fn when(ts: i64) -> String {
    let secs = (shadow_core::activity::now_ms() - ts).max(0) / 1000;
    match secs {
        0..=44 => "just now".into(),
        45..=3599 => format!("{}m ago", (secs + 30) / 60),
        3600..=86399 => format!("{}h ago", (secs + 1800) / 3600),
        _ => chrono::DateTime::from_timestamp_millis(ts)
            .map(|d| d.with_timezone(&chrono::Local).format("%b %d").to_string())
            .unwrap_or_default(),
    }
}

fn action_span(a: Action) -> Span<'static> {
    match a {
        Action::Masked => Span::styled(" MASKED ", Style::new().fg(Color::White).bg(ACCENT).bold()),
        Action::Blocked => {
            Span::styled(" BLOCKED ", Style::new().fg(Color::White).bg(DANGER).bold())
        }
        Action::Clean => Span::styled(" CLEAN ", Style::new().fg(MUTED)),
    }
}

fn reason_label(r: Option<&str>) -> String {
    shadow_core::activity::block_reason_label(r.unwrap_or("file_policy"))
}

fn draw_incidents(f: &mut Frame, area: Rect, app: &mut App) {
    let (list_area, detail_area) = if app.detail {
        let [a, b] = Layout::horizontal([Constraint::Percentage(58), Constraint::Percentage(42)])
            .areas(area);
        (a, Some(b))
    } else {
        (area, None)
    };
    let rows: Vec<Row> = app
        .snap
        .incidents
        .iter()
        .map(|e| {
            let what = if e.action == Action::Blocked {
                Span::styled(
                    reason_label(e.block_reason.as_deref()),
                    Style::new().fg(DANGER),
                )
            } else {
                Span::styled(describe_kinds(e, 3), Style::new().fg(ACCENT_SOFT))
            };
            Row::new(vec![
                Cell::from(Span::styled(when(e.ts), Style::new().fg(MUTED))),
                Cell::from(Line::from(vec![
                    Span::styled(e.client.clone(), Style::new().bold()),
                    Span::styled(" → ", Style::new().fg(MUTED)),
                    Span::raw(provider_label(&e.provider)),
                ])),
                Cell::from(what),
                Cell::from(action_span(e.action)),
                Cell::from(if e.restored > 0 {
                    Span::styled(format!("↩ {}", e.restored), Style::new().fg(OK))
                } else {
                    Span::styled("—", Style::new().fg(MUTED))
                }),
            ])
        })
        .collect();
    let empty = rows.is_empty();
    let table = Table::new(
        rows,
        [
            Constraint::Length(9),
            Constraint::Percentage(40),
            Constraint::Fill(1),
            Constraint::Length(9),
            Constraint::Length(6),
        ],
    )
    .header(
        Row::new(["WHEN", "SOURCE → DESTINATION", "WHAT", "ACTION", "BACK"])
            .style(Style::new().fg(MUTED).bold()),
    )
    .row_highlight_style(Style::new().bg(Color::Rgb(40, 36, 70)))
    .highlight_symbol("▌")
    .block(card("Recent incidents · masked or blocked requests"));
    f.render_stateful_widget(table, list_area, &mut app.table);
    if empty {
        let msg = Paragraph::new(
            "No incidents yet. When Shadow masks or blocks something, it shows up here.",
        )
        .fg(MUTED)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
        let inner = list_area.inner(ratatui::layout::Margin::new(2, 3));
        f.render_widget(msg, inner);
    }
    if let (Some(d), Some(i)) = (detail_area, app.table.selected()) {
        if let Some(e) = app.snap.incidents.get(i) {
            draw_detail(f, d, e);
        }
    }
}

fn draw_detail(f: &mut Frame, area: Rect, e: &Event) {
    f.render_widget(Clear, area);
    let kv = |k: &str, v: String| {
        Line::from(vec![
            Span::styled(format!("{k:<12}"), Style::new().fg(MUTED)),
            Span::raw(v),
        ])
    };
    let time = chrono::DateTime::from_timestamp_millis(e.ts)
        .map(|d| {
            d.with_timezone(&chrono::Local)
                .format("%a %b %d %H:%M:%S")
                .to_string()
        })
        .unwrap_or_default();
    let mut lines = vec![
        Line::from(vec![
            action_span(e.action),
            Span::raw("  "),
            Span::raw(time),
        ]),
        Line::raw(""),
        kv("App", e.client.clone()),
        kv(
            "Destination",
            format!("{} ({})", provider_label(&e.provider), e.host),
        ),
        kv("Endpoint", e.endpoint.as_str().into()),
        kv(
            "Route",
            if e.mode == "gateway" {
                "gateway (/gw)".into()
            } else {
                "HTTPS proxy".into()
            },
        ),
    ];
    if let Some(i) = &e.intent {
        lines.push(kv("Intent", i.clone()));
    }
    if e.action == Action::Blocked {
        lines.push(kv("Reason", reason_label(e.block_reason.as_deref())));
    }
    for (k, n) in &e.kinds {
        lines.push(kv(
            "Protected",
            format!("{n} × {}", shadow_core::activity::kind_label(k)),
        ));
    }
    if !e.detectors.is_empty() {
        lines.push(kv("Detectors", e.detectors.join(", ")));
    }
    lines.push(kv("Restored", format!("{} value(s)", e.restored)));
    lines.push(kv(
        "Response",
        format!(
            "HTTP {}{}",
            e.status,
            if e.streamed { " · streamed" } else { "" }
        ),
    ));
    lines.push(kv(
        "Size",
        format!(
            "{} B sent · {} B received",
            fmt_n(e.bytes_in),
            fmt_n(e.bytes_out)
        ),
    ));
    lines.push(kv("Latency", format!("{} ms", fmt_n(e.latency_ms))));
    lines.push(Line::raw(""));
    lines.push(Line::styled(
        "Never recorded: real values, fakes, prompts, file names, headers.",
        Style::new().fg(OK),
    ));
    f.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(card("Incident · esc to close")),
        area,
    );
}

fn draw_breakdown(f: &mut Frame, area: Rect, app: &App) {
    let s = &app.snap.summary;
    let [top, bottom] =
        Layout::vertical([Constraint::Percentage(60), Constraint::Percentage(40)]).areas(area);
    let [a, b] =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(top);
    let [c, d, e] = Layout::horizontal([
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
    ])
    .areas(bottom);
    bar_list(
        f,
        a,
        "Values protected by data type",
        &s.kinds,
        |r| r.values,
        ACCENT,
    );
    bar_list(
        f,
        b,
        "Requests by app",
        &s.clients,
        |r| r.requests,
        ACCENT_SOFT,
    );
    let providers: Vec<Breakdown> = s
        .providers
        .iter()
        .map(|p| Breakdown {
            label: provider_label(&p.key),
            ..p.clone()
        })
        .collect();
    bar_list(
        f,
        c,
        "Requests by destination",
        &providers,
        |r| r.requests,
        OK,
    );
    bar_list(f, d, "Endpoints", &s.endpoints, |r| r.requests, MUTED);
    bar_list(
        f,
        e,
        "Blocked by reason",
        &s.block_reasons,
        |r| r.requests,
        DANGER,
    );
}

/// Render one frame to a string (used by tests and `--demo` smoke checks).
/// One bar of a timeline chart, without its value printed underneath.
fn bar(value: u64, color: Color) -> Bar<'static> {
    Bar::default()
        .value(value)
        .text_value(String::new())
        .style(Style::new().fg(color))
}

#[cfg(test)]
fn render_to_string(app: &mut App, w: u16, h: u16) -> String {
    use ratatui::backend::TestBackend;
    let mut term = ratatui::Terminal::new(TestBackend::new(w, h)).unwrap();
    term.draw(|f| draw(f, app)).unwrap();
    let buf = term.backend().buffer().clone();
    let mut out = String::new();
    for y in 0..h {
        for x in 0..w {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(source: Source) -> App {
        let snap = source.fetch(Window::Day, 200);
        App {
            source,
            window: Window::Day,
            tab: 0,
            snap,
            table: TableState::default().with_selected(Some(0)),
            detail: false,
            last: Instant::now(),
        }
    }

    #[test]
    fn demo_renders_every_tab() {
        let mut a = app(Source::demo());
        let s = render_to_string(&mut a, 120, 40);
        if std::env::var_os("SHADOW_PRINT_TOP").is_some() {
            println!("{s}");
        }
        assert!(s.contains("DEMO DATA"));
        assert!(s.contains("Values protected"));
        assert!(s.contains("Anthropic"));
        assert!(s.contains("Claude Code"));
        a.tab = 1;
        a.detail = true;
        let s = render_to_string(&mut a, 120, 40);
        if std::env::var_os("SHADOW_PRINT_TOP").is_some() {
            println!("{s}");
        }
        assert!(s.contains("Recent incidents"));
        assert!(s.contains("Never recorded"));
        a.tab = 2;
        let s = render_to_string(&mut a, 80, 24);
        assert!(s.contains("Endpoints"));
    }

    #[test]
    fn offline_degrades_gracefully() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = shadow_core::ShadowConfig::from_dir(dir.path());
        cfg.proxy.listen = "127.0.0.1:9".into(); // discard port: nothing listens
        let mut a = app(Source::live(&cfg));
        assert!(matches!(a.snap.state, Feed::Offline(_)));
        let s = render_to_string(&mut a, 100, 30);
        assert!(s.contains("Proxy offline"));
        assert!(s.contains("shadow run"));
    }
}
