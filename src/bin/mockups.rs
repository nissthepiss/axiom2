//! TUI Style Explorer — 10 layout mockups with fake data
//! Navigate with Left/Right arrows, quit with q/Esc

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    prelude::*,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::*,
    Frame,
};
use std::io::stdout;

// ── Colors ──────────────────────────────────────────────────
const ORANGE: Color = Color::Rgb(255, 165, 0);
const DIM: Color = Color::Rgb(90, 90, 90);
const SURFACE: Color = Color::Rgb(22, 22, 30);
const SURFACE2: Color = Color::Rgb(28, 28, 40);
const SURFACE3: Color = Color::Rgb(35, 35, 50);
const TEXT: Color = Color::Rgb(200, 200, 210);
const TEXT_DIM: Color = Color::Rgb(120, 120, 140);
const GREEN: Color = Color::Rgb(38, 166, 91);
const RED: Color = Color::Rgb(234, 57, 67);
const CYAN: Color = Color::Rgb(80, 200, 255);
const YELLOW: Color = Color::Rgb(255, 220, 80);
const PURPLE: Color = Color::Rgb(160, 100, 255);
const GRID: Color = Color::Rgb(40, 40, 55);
const BLUE: Color = Color::Rgb(60, 130, 246);

// ── Fake data ───────────────────────────────────────────────
struct FakeTrade {
    time: &'static str,
    is_buy: bool,
    sol: f64,
    wallet: &'static str,
    mcap: &'static str,
}

const TRADES: &[FakeTrade] = &[
    FakeTrade { time: "14:23:01", is_buy: true,  sol: 12.450, wallet: "Degen...xB9f", mcap: "$128.4k" },
    FakeTrade { time: "14:23:01", is_buy: true,  sol: 3.200,  wallet: "Whale...7pmQ", mcap: "$127.9k" },
    FakeTrade { time: "14:23:00", is_buy: false, sol: 0.890,  wallet: "BoT3x...aK1p", mcap: "$126.1k" },
    FakeTrade { time: "14:22:59", is_buy: true,  sol: 24.100, wallet: "AlphA...ZZz0", mcap: "$131.2k" },
    FakeTrade { time: "14:22:59", is_buy: true,  sol: 0.045,  wallet: "fresh...9w2K", mcap: "$130.8k" },
    FakeTrade { time: "14:22:58", is_buy: false, sol: 5.670,  wallet: "RugPu...LMAO", mcap: "$125.3k" },
    FakeTrade { time: "14:22:58", is_buy: true,  sol: 1.230,  wallet: "SoLan...DAo1", mcap: "$126.7k" },
    FakeTrade { time: "14:22:57", is_buy: true,  sol: 0.510,  wallet: "pump3...rKt4", mcap: "$126.2k" },
    FakeTrade { time: "14:22:57", is_buy: false, sol: 8.900,  wallet: "Sn1pR...Kill", mcap: "$121.0k" },
    FakeTrade { time: "14:22:56", is_buy: true,  sol: 0.098,  wallet: "NooB1...fiRs", mcap: "$121.8k" },
    FakeTrade { time: "14:22:56", is_buy: true,  sol: 2.340,  wallet: "chad0...xMax", mcap: "$123.1k" },
    FakeTrade { time: "14:22:55", is_buy: false, sol: 0.340,  wallet: "paper...hNds", mcap: "$122.5k" },
    FakeTrade { time: "14:22:55", is_buy: true,  sol: 6.780,  wallet: "DiaM0...Hnd5", mcap: "$124.9k" },
    FakeTrade { time: "14:22:54", is_buy: true,  sol: 0.023,  wallet: "sMolB...uyEr", mcap: "$124.7k" },
    FakeTrade { time: "14:22:54", is_buy: false, sol: 1.110,  wallet: "ExitL...iq00", mcap: "$123.8k" },
    FakeTrade { time: "14:22:53", is_buy: true,  sol: 4.560,  wallet: "ApeIn...NoW!", mcap: "$125.6k" },
    FakeTrade { time: "14:22:52", is_buy: true,  sol: 0.780,  wallet: "Normie..xYz", mcap: "$125.2k" },
    FakeTrade { time: "14:22:51", is_buy: false, sol: 15.200, wallet: "DUMP!...rEKt", mcap: "$118.3k" },
    FakeTrade { time: "14:22:50", is_buy: true,  sol: 1.890,  wallet: "buLLi...sH99", mcap: "$120.1k" },
    FakeTrade { time: "14:22:49", is_buy: true,  sol: 0.340,  wallet: "anon4...2day", mcap: "$119.7k" },
];

// Candlestick data (open, high, low, close)
const CANDLES: &[(f64, f64, f64, f64)] = &[
    (118.0, 120.5, 117.2, 119.8), (119.8, 121.0, 118.5, 118.9),
    (118.9, 119.5, 116.0, 116.5), (116.5, 118.2, 115.8, 117.9),
    (117.9, 122.0, 117.5, 121.3), (121.3, 123.5, 120.8, 122.1),
    (122.1, 122.8, 119.0, 119.5), (119.5, 121.0, 118.0, 120.8),
    (120.8, 124.0, 120.2, 123.5), (123.5, 125.0, 122.0, 122.8),
    (122.8, 123.2, 120.5, 121.0), (121.0, 121.5, 118.3, 119.2),
    (119.2, 120.8, 118.0, 120.5), (120.5, 126.0, 120.0, 125.5),
    (125.5, 128.0, 124.5, 127.2), (127.2, 129.5, 126.0, 128.4),
];

const STYLES: &[(&str, &str)] = &[
    ("CURRENT+",       "Classic layout. Trades left, chart + stats right. Clean and functional."),
    ("CHART KING",     "Chart takes 60% of screen. Trades compressed below. Price action focus."),
    ("Bloomberg",      "4-panel grid. Chart, trades, orderflow, stats. Maximum information density."),
    ("MINIMAL ZEN",    "Just trades + price. No borders, no noise. Pure data."),
    ("WHALE HUNTER",   "Trades sorted by size. Whale alerts prominent. Big money visibility."),
    ("HEATMAP",        "Volume heatmap by time bucket. Visual pattern recognition."),
    ("TAPE READER",    "Full-width trade tape. Orderflow focus. Speed-reading layout."),
    ("SPLIT CHART",    "Two timeframes side by side (5s + 1m). Multi-timeframe analysis."),
    ("STRATEGY VIEW",  "Paper trading overlay. Entry/exit signals, P&L tracking, position sizing."),
    ("DARK TERMINAL",  "Retro terminal aesthetic. Monochrome green. Hacker vibes."),
];

fn main() -> anyhow::Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut current = 0usize;

    loop {
        terminal.draw(|f| draw(f, current))?;

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Right | KeyCode::Char('l') => current = (current + 1) % STYLES.len(),
                        KeyCode::Left  | KeyCode::Char('h') => current = (current + STYLES.len() - 1) % STYLES.len(),
                        _ => {}
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn draw(f: &mut Frame, style_idx: usize) {
    let area = f.area();

    // Fill background
    f.render_widget(Block::default().style(Style::default().bg(SURFACE)), area);

    // Layout: nav bar (2) + body + description bar (2)
    let outer = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(10),
        Constraint::Length(2),
    ]).split(area);

    draw_nav_bar(f, outer[0], style_idx);
    draw_description_bar(f, outer[2], style_idx);

    match style_idx {
        0 => draw_style_current(f, outer[1]),
        1 => draw_style_chart_king(f, outer[1]),
        2 => draw_style_bloomberg(f, outer[1]),
        3 => draw_style_minimal(f, outer[1]),
        4 => draw_style_whale_hunter(f, outer[1]),
        5 => draw_style_heatmap(f, outer[1]),
        6 => draw_style_tape_reader(f, outer[1]),
        7 => draw_style_split_chart(f, outer[1]),
        8 => draw_style_strategy(f, outer[1]),
        9 => draw_style_dark_terminal(f, outer[1]),
        _ => {}
    }
}

// ── Navigation bar ──────────────────────────────────────────
fn draw_nav_bar(f: &mut Frame, area: Rect, current: usize) {
    let mut spans = vec![
        Span::styled("  ", Style::default()),
        Span::styled("◄ ", Style::default().fg(ORANGE)),
    ];

    for (i, (name, _)) in STYLES.iter().enumerate() {
        if i == current {
            spans.push(Span::styled(
                format!(" {} ", name),
                Style::default().fg(Color::White).bg(ORANGE).add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                format!(" {} ", name),
                Style::default().fg(TEXT_DIM),
            ));
        }
    }

    spans.push(Span::styled(" ►", Style::default().fg(ORANGE)));

    let nav = Paragraph::new(Line::from(spans)).style(Style::default().bg(SURFACE2));
    f.render_widget(nav, area);
}

fn draw_description_bar(f: &mut Frame, area: Rect, current: usize) {
    let (_name, desc) = STYLES[current];
    let line = Line::from(vec![
        Span::styled(format!("  {}/{} ", current + 1, STYLES.len()), Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)),
        Span::styled("│ ", Style::default().fg(DIM)),
        Span::styled(desc, Style::default().fg(TEXT_DIM)),
        Span::styled("  │  ←/→ navigate  q quit", Style::default().fg(DIM)),
    ]);
    let bar = Paragraph::new(line).style(Style::default().bg(SURFACE2));
    f.render_widget(bar, area);
}

// ── Shared helpers ──────────────────────────────────────────

fn fake_header(f: &mut Frame, area: Rect) {
    let row1 = Line::from(vec![
        Span::styled(" AXIOM", Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)),
        Span::styled(" TELEMETRY ", Style::default().fg(TEXT_DIM)),
        Span::styled("●", Style::default().fg(GREEN)),
        Span::styled(" LIVE", Style::default().fg(GREEN).add_modifier(Modifier::BOLD)),
    ]);
    let row2 = Line::from(vec![
        Span::styled(" PUMP", Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)),
        Span::styled("  Moon Mission Token  ", Style::default().fg(TEXT)),
        Span::styled("7vaeM...pump", Style::default().fg(TEXT_DIM)),
    ]);
    let row3 = Line::from(vec![
        Span::styled(" Market Cap ", Style::default().fg(TEXT_DIM)),
        Span::styled("$128.4k", Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)),
        Span::styled("    SOL ", Style::default().fg(TEXT_DIM)),
        Span::styled("$140.23", Style::default().fg(TEXT)),
        Span::styled("    Txns ", Style::default().fg(TEXT_DIM)),
        Span::styled("1,847", Style::default().fg(TEXT)),
        Span::styled("    Wallets ", Style::default().fg(TEXT_DIM)),
        Span::styled("312", Style::default().fg(TEXT)),
    ]);
    let p = Paragraph::new(vec![row1, row2, row3]).style(Style::default().bg(SURFACE));
    f.render_widget(p, area);
}

fn render_trades(f: &mut Frame, area: Rect, count: usize) {
    let block = Block::default()
        .title(Span::styled(" TRADES ", Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(DIM))
        .style(Style::default().bg(SURFACE))
        .padding(Padding::new(1, 1, 0, 0));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let w = inner.width as usize;
    let mut lines = Vec::new();

    lines.push(Line::from(vec![
        Span::styled("TIME     ", Style::default().fg(TEXT_DIM)),
        Span::styled("SIDE  ", Style::default().fg(TEXT_DIM)),
        Span::styled("     AMOUNT ", Style::default().fg(TEXT_DIM)),
        Span::styled("    WALLET     ", Style::default().fg(TEXT_DIM)),
        Span::styled("    MCAP", Style::default().fg(TEXT_DIM)),
    ]));
    lines.push(Line::from(Span::styled("─".repeat(w.min(70)), Style::default().fg(DIM))));

    for t in TRADES.iter().take(count) {
        let (side, style) = if t.is_buy {
            ("BUY ", Style::default().fg(GREEN).add_modifier(Modifier::BOLD))
        } else {
            ("SELL", Style::default().fg(RED).add_modifier(Modifier::BOLD))
        };
        let amt_style = if t.sol >= 5.0 {
            Style::default().fg(YELLOW).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(TEXT).add_modifier(Modifier::BOLD)
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{} ", t.time), Style::default().fg(TEXT_DIM)),
            Span::styled(format!("{}  ", side), style),
            Span::styled(format!("{:>8.3} SOL ", t.sol), amt_style),
            Span::styled(format!("  {:^13} ", t.wallet), Style::default().fg(TEXT_DIM)),
            Span::styled(format!("{:>8}", t.mcap), Style::default().fg(TEXT_DIM)),
        ]));
    }

    f.render_widget(Paragraph::new(lines).style(Style::default().bg(SURFACE)), inner);
}

fn render_candles(buf: &mut Buffer, area: Rect, candles: &[(f64, f64, f64, f64)]) {
    if area.height < 3 || area.width < 12 { return; }

    // Fill bg
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            buf[(x, y)].set_style(Style::default().bg(SURFACE));
        }
    }

    let axis_w: u16 = 8;
    let cw = area.width.saturating_sub(axis_w) as usize;
    let ch = area.height as usize;

    let low = candles.iter().map(|c| c.2).fold(f64::INFINITY, f64::min);
    let high = candles.iter().map(|c| c.1).fold(f64::NEG_INFINITY, f64::max);
    let range = (high - low).max(0.1);
    let pad = range * 0.1;
    let ymin = low - pad;
    let ymax = high + pad;
    let yr = (ymax - ymin).max(0.01);

    let val_to_row = |v: f64| -> usize {
        let frac = (v - ymin) / yr;
        ((1.0 - frac) * (ch as f64 - 1.0)).round() as usize
    };

    // Grid
    let axis_x = area.x + cw as u16;
    for gi in 0..=3 {
        let row = area.y + (gi as u16 * area.height.saturating_sub(1)) / 3;
        if row < area.y + area.height {
            for x in area.x..axis_x {
                if (x - area.x) % 4 == 0 {
                    buf[(x, row)].set_char('·');
                    buf[(x, row)].set_fg(GRID);
                }
            }
            let val = ymax - (gi as f64 / 3.0) * yr;
            let label = format!("${:.1}k", val);
            for (ci, ch) in label.chars().enumerate() {
                let px = axis_x + 1 + ci as u16;
                if px < area.x + area.width {
                    buf[(px, row)].set_char(ch);
                    buf[(px, row)].set_fg(TEXT_DIM);
                }
            }
        }
    }

    // Separator
    for y in area.y..area.y + area.height {
        if axis_x < area.x + area.width {
            buf[(axis_x, y)].set_char('│');
            buf[(axis_x, y)].set_fg(DIM);
        }
    }

    // Candles
    let n = candles.len();
    let pitch = cw as f64 / n as f64;
    let bw = ((pitch - 1.0).floor() as usize).max(1).min(5);
    let bw = if bw > 1 && bw % 2 == 0 { bw - 1 } else { bw };

    for (i, &(o, h, l, c)) in candles.iter().enumerate() {
        let cx = area.x + ((i as f64 + 0.5) * pitch) as u16;
        if cx >= axis_x { break; }

        let color = if c >= o { GREEN } else { RED };
        let rh = val_to_row(h);
        let rl = val_to_row(l);
        let ro = val_to_row(o);
        let rc = val_to_row(c);
        let bt = ro.min(rc);
        let bb = ro.max(rc);

        for row in rh..=rl {
            let py = area.y + row as u16;
            if py < area.y + area.height { buf[(cx, py)].set_char('│'); buf[(cx, py)].set_fg(color); }
        }
        let hw = bw / 2;
        let bs = cx.saturating_sub(hw as u16);
        let be = (cx + hw as u16 + 1).min(axis_x);
        for row in bt..=bb {
            let py = area.y + row as u16;
            if py >= area.y + area.height { continue; }
            for px in bs..be {
                if px >= area.x && px < axis_x {
                    buf[(px, py)].set_char('█'); buf[(px, py)].set_fg(color); buf[(px, py)].set_bg(SURFACE);
                }
            }
        }
    }

    // Current price marker
    if let Some(&(_, _, _, c)) = candles.last() {
        let row = val_to_row(c);
        let py = area.y + row as u16;
        let color = if candles.last().map(|c| c.3 >= c.0).unwrap_or(true) { GREEN } else { RED };
        if py < area.y + area.height && axis_x < area.x + area.width {
            buf[(axis_x, py)].set_char('◄'); buf[(axis_x, py)].set_fg(color);
            let label = format!("${:.1}k", c);
            for (ci, ch) in label.chars().enumerate() {
                let px = axis_x + 1 + ci as u16;
                if px < area.x + area.width {
                    buf[(px, py)].set_char(ch); buf[(px, py)].set_fg(Color::White); buf[(px, py)].set_bg(color);
                }
            }
        }
    }
}

fn render_volume_bars(f: &mut Frame, area: Rect) {
    let mut lines = Vec::new();
    lines.push(Line::from(Span::styled(" VOLUME", Style::default().fg(CYAN).add_modifier(Modifier::BOLD))));
    let bw = area.width.saturating_sub(6) as usize;
    let buy_len = (bw as f64 * 0.68) as usize;
    let sell_len = bw.saturating_sub(buy_len);
    lines.push(Line::from(vec![
        Span::styled(" 1m ", Style::default().fg(TEXT_DIM)),
        Span::styled("14.2", Style::default().fg(TEXT).add_modifier(Modifier::BOLD)),
        Span::styled(" SOL   ", Style::default().fg(TEXT_DIM)),
        Span::styled("5m ", Style::default().fg(TEXT_DIM)),
        Span::styled("58.7", Style::default().fg(TEXT).add_modifier(Modifier::BOLD)),
        Span::styled(" SOL", Style::default().fg(TEXT_DIM)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" ", Style::default()),
        Span::styled("█".repeat(buy_len), Style::default().fg(GREEN)),
        Span::styled("█".repeat(sell_len), Style::default().fg(RED)),
        Span::styled(" 68%B", Style::default().fg(TEXT_DIM)),
    ]));
    f.render_widget(Paragraph::new(lines).style(Style::default().bg(SURFACE)), area);
}

fn render_momentum(f: &mut Frame, area: Rect) {
    let mut lines = Vec::new();
    lines.push(Line::from(Span::styled(" MOMENTUM", Style::default().fg(CYAN).add_modifier(Modifier::BOLD))));
    lines.push(Line::from(vec![
        Span::styled(" Pressure ", Style::default().fg(TEXT_DIM)),
        Span::styled("STRONG BUY", Style::default().fg(GREEN).add_modifier(Modifier::BOLD)),
    ]));
    let bw = area.width.saturating_sub(8) as usize;
    let bl = (bw as f64 * 0.78) as usize;
    lines.push(Line::from(vec![
        Span::styled(" ", Style::default()),
        Span::styled("█".repeat(bl), Style::default().fg(GREEN)),
        Span::styled("█".repeat(bw.saturating_sub(bl)), Style::default().fg(RED)),
        Span::styled(" 78%B", Style::default().fg(TEXT_DIM)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" Vol Δ    ", Style::default().fg(TEXT_DIM)),
        Span::styled("SURGING (2.4x)", Style::default().fg(GREEN).add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" TPS ", Style::default().fg(TEXT_DIM)),
        Span::styled("3.8", Style::default().fg(TEXT)),
        Span::styled("   5m ", Style::default().fg(TEXT_DIM)),
        Span::styled("142", Style::default().fg(GREEN)),
        Span::styled("B ", Style::default().fg(TEXT_DIM)),
        Span::styled("67", Style::default().fg(RED)),
        Span::styled("S", Style::default().fg(TEXT_DIM)),
        Span::styled("   Lg ", Style::default().fg(TEXT_DIM)),
        Span::styled("24.10", Style::default().fg(YELLOW)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" Fees ", Style::default().fg(TEXT_DIM)),
        Span::styled("4.8720 SOL", Style::default().fg(ORANGE)),
    ]));
    f.render_widget(Paragraph::new(lines).style(Style::default().bg(SURFACE)), area);
}

fn status_bar(f: &mut Frame, area: Rect) {
    let line = Line::from(vec![
        Span::styled(" ", Style::default()),
        Span::styled("●", Style::default().fg(GREEN)),
        Span::styled(" Connected", Style::default().fg(TEXT_DIM)),
        Span::styled("  │  ", Style::default().fg(DIM)),
        Span::styled("Last update ", Style::default().fg(TEXT_DIM)),
        Span::styled("just now", Style::default().fg(TEXT)),
        Span::styled("  │  ", Style::default().fg(DIM)),
        Span::styled("Uptime ", Style::default().fg(TEXT_DIM)),
        Span::styled("12m 34s", Style::default().fg(TEXT)),
        Span::styled("  │  ", Style::default().fg(DIM)),
        Span::styled("q", Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)),
        Span::styled(" quit  ", Style::default().fg(TEXT_DIM)),
        Span::styled("t", Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)),
        Span::styled(" timeframe", Style::default().fg(TEXT_DIM)),
    ]);
    f.render_widget(Paragraph::new(line).style(Style::default().bg(SURFACE2)), area);
}

// ═══════════════════════════════════════════════════════════
// STYLE 0: CURRENT+ (improved version of current layout)
// ═══════════════════════════════════════════════════════════
fn draw_style_current(f: &mut Frame, area: Rect) {
    let rows = Layout::vertical([
        Constraint::Length(3), Constraint::Min(8), Constraint::Length(1),
    ]).split(area);

    fake_header(f, rows[0]);
    status_bar(f, rows[2]);

    let cols = Layout::horizontal([
        Constraint::Percentage(58), Constraint::Percentage(42),
    ]).split(rows[1]);

    render_trades(f, cols[0], 18);

    let right_block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(DIM)).style(Style::default().bg(SURFACE));
    let right_inner = right_block.inner(cols[1]);
    f.render_widget(right_block, cols[1]);

    let sections = Layout::vertical([
        Constraint::Min(6), Constraint::Length(3), Constraint::Length(6),
    ]).split(right_inner);

    // Chart title
    let title = Paragraph::new(Line::from(vec![
        Span::styled(" MC", Style::default().fg(CYAN).add_modifier(Modifier::BOLD)),
        Span::styled("  1m", Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)),
        Span::styled("  t", Style::default().fg(TEXT_DIM)),
    ])).style(Style::default().bg(SURFACE));
    let chart_title = Rect { x: sections[0].x, y: sections[0].y, width: sections[0].width, height: 1 };
    f.render_widget(title, chart_title);
    let chart_area = Rect { x: sections[0].x, y: sections[0].y + 1, width: sections[0].width, height: sections[0].height.saturating_sub(1) };
    render_candles(f.buffer_mut(), chart_area, CANDLES);

    render_volume_bars(f, sections[1]);
    render_momentum(f, sections[2]);
}

// ═══════════════════════════════════════════════════════════
// STYLE 1: CHART KING
// ═══════════════════════════════════════════════════════════
fn draw_style_chart_king(f: &mut Frame, area: Rect) {
    let rows = Layout::vertical([
        Constraint::Length(3), Constraint::Percentage(60), Constraint::Percentage(40), Constraint::Length(1),
    ]).split(area);

    fake_header(f, rows[0]);
    status_bar(f, rows[3]);

    // Big chart
    let chart_block = Block::default()
        .title(Line::from(vec![
            Span::styled(" MARKET CAP ", Style::default().fg(CYAN).add_modifier(Modifier::BOLD)),
            Span::styled("1m", Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)),
        ]))
        .borders(Borders::ALL).border_style(Style::default().fg(DIM)).style(Style::default().bg(SURFACE));
    let chart_inner = chart_block.inner(rows[1]);
    f.render_widget(chart_block, rows[1]);
    render_candles(f.buffer_mut(), chart_inner, CANDLES);

    // Bottom: trades + stats
    let bottom = Layout::horizontal([
        Constraint::Percentage(60), Constraint::Percentage(40),
    ]).split(rows[2]);

    render_trades(f, bottom[0], 8);

    let stats_block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(DIM)).style(Style::default().bg(SURFACE));
    let stats_inner = stats_block.inner(bottom[1]);
    f.render_widget(stats_block, bottom[1]);

    let stats_parts = Layout::vertical([
        Constraint::Length(3), Constraint::Min(4),
    ]).split(stats_inner);
    render_volume_bars(f, stats_parts[0]);
    render_momentum(f, stats_parts[1]);
}

// ═══════════════════════════════════════════════════════════
// STYLE 2: BLOOMBERG (4-panel grid)
// ═══════════════════════════════════════════════════════════
fn draw_style_bloomberg(f: &mut Frame, area: Rect) {
    let rows = Layout::vertical([
        Constraint::Length(3), Constraint::Min(8), Constraint::Length(1),
    ]).split(area);
    fake_header(f, rows[0]);
    status_bar(f, rows[2]);

    let top_bottom = Layout::vertical([
        Constraint::Percentage(50), Constraint::Percentage(50),
    ]).split(rows[1]);

    let top_cols = Layout::horizontal([
        Constraint::Percentage(50), Constraint::Percentage(50),
    ]).split(top_bottom[0]);

    let bot_cols = Layout::horizontal([
        Constraint::Percentage(50), Constraint::Percentage(50),
    ]).split(top_bottom[1]);

    // Top-left: chart
    let cb = Block::default().title(Span::styled(" CHART ", Style::default().fg(CYAN).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL).border_style(Style::default().fg(DIM)).style(Style::default().bg(SURFACE));
    let ci = cb.inner(top_cols[0]);
    f.render_widget(cb, top_cols[0]);
    render_candles(f.buffer_mut(), ci, CANDLES);

    // Top-right: trades
    render_trades(f, top_cols[1], 10);

    // Bottom-left: orderflow
    let of_block = Block::default().title(Span::styled(" ORDER FLOW ", Style::default().fg(PURPLE).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL).border_style(Style::default().fg(DIM)).style(Style::default().bg(SURFACE)).padding(Padding::new(1, 1, 0, 0));
    let of_inner = of_block.inner(bot_cols[0]);
    f.render_widget(of_block, bot_cols[0]);

    let mut of_lines = Vec::new();
    of_lines.push(Line::from(vec![
        Span::styled("PRICE LVL    ", Style::default().fg(TEXT_DIM)),
        Span::styled("BUY VOL     SELL VOL", Style::default().fg(TEXT_DIM)),
    ]));
    let levels = [
        ("$130k+", 2.1, 0.5), ("$128-130k", 8.4, 1.2), ("$126-128k", 12.3, 3.4),
        ("$124-126k", 6.7, 5.1), ("$122-124k", 3.2, 8.9), ("$120-122k", 1.8, 14.2),
        ("$118-120k", 0.9, 6.3), ("<$118k", 0.3, 2.1),
    ];
    for (label, buy, sell) in levels {
        let max_bar = 12;
        let max_vol = 15.0;
        let bl = ((buy / max_vol) * max_bar as f64).round() as usize;
        let sl = ((sell / max_vol) * max_bar as f64).round() as usize;
        of_lines.push(Line::from(vec![
            Span::styled(format!("{:<13}", label), Style::default().fg(TEXT_DIM)),
            Span::styled("█".repeat(bl.min(max_bar)), Style::default().fg(GREEN)),
            Span::styled("░".repeat(max_bar.saturating_sub(bl)), Style::default().fg(GRID)),
            Span::styled(" ", Style::default()),
            Span::styled("█".repeat(sl.min(max_bar)), Style::default().fg(RED)),
            Span::styled("░".repeat(max_bar.saturating_sub(sl)), Style::default().fg(GRID)),
        ]));
    }
    f.render_widget(Paragraph::new(of_lines).style(Style::default().bg(SURFACE)), of_inner);

    // Bottom-right: stats
    let sb = Block::default().title(Span::styled(" STATS ", Style::default().fg(YELLOW).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL).border_style(Style::default().fg(DIM)).style(Style::default().bg(SURFACE));
    let si = sb.inner(bot_cols[1]);
    f.render_widget(sb, bot_cols[1]);
    let sp = Layout::vertical([Constraint::Length(3), Constraint::Min(4)]).split(si);
    render_volume_bars(f, sp[0]);
    render_momentum(f, sp[1]);
}

// ═══════════════════════════════════════════════════════════
// STYLE 3: MINIMAL ZEN
// ═══════════════════════════════════════════════════════════
fn draw_style_minimal(f: &mut Frame, area: Rect) {
    let rows = Layout::vertical([
        Constraint::Length(3), Constraint::Min(8),
    ]).split(area);

    // Ultra-minimal header
    let header = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("    PUMP", Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)),
            Span::styled("  $128.4k", Style::default().fg(TEXT).add_modifier(Modifier::BOLD)),
            Span::styled("  ▲ 12.3%", Style::default().fg(GREEN).add_modifier(Modifier::BOLD)),
            Span::styled("                                ", Style::default()),
            Span::styled("●", Style::default().fg(GREEN)),
            Span::styled(" 1,847 txns", Style::default().fg(TEXT_DIM)),
        ]),
    ]).style(Style::default().bg(SURFACE));
    f.render_widget(header, rows[0]);

    // Just trades, no borders, no chrome
    let mut lines = Vec::new();
    lines.push(Line::from(""));
    for t in TRADES.iter().take(area.height.saturating_sub(5) as usize) {
        let color = if t.is_buy { GREEN } else { RED };
        let side = if t.is_buy { "+" } else { "-" };
        let whale = if t.sol >= 5.0 { " 🐋" } else { "" };
        lines.push(Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled(t.time, Style::default().fg(TEXT_DIM)),
            Span::styled(format!("  {}{:.3} SOL", side, t.sol), Style::default().fg(color).add_modifier(Modifier::BOLD)),
            Span::styled(format!("  {}", t.wallet), Style::default().fg(TEXT_DIM)),
            Span::styled(format!("  {}", t.mcap), Style::default().fg(DIM)),
            Span::styled(whale, Style::default()),
        ]));
    }
    f.render_widget(Paragraph::new(lines).style(Style::default().bg(SURFACE)), rows[1]);
}

// ═══════════════════════════════════════════════════════════
// STYLE 4: WHALE HUNTER
// ═══════════════════════════════════════════════════════════
fn draw_style_whale_hunter(f: &mut Frame, area: Rect) {
    let rows = Layout::vertical([
        Constraint::Length(3), Constraint::Length(5), Constraint::Min(8), Constraint::Length(1),
    ]).split(area);
    fake_header(f, rows[0]);
    status_bar(f, rows[3]);

    // Whale alert banner
    let alert_block = Block::default()
        .title(Span::styled(" WHALE ALERTS ", Style::default().fg(YELLOW).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL).border_style(Style::default().fg(YELLOW)).style(Style::default().bg(SURFACE3));
    let alert_inner = alert_block.inner(rows[1]);
    f.render_widget(alert_block, rows[1]);
    let alerts = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(" 14:22:59  ", Style::default().fg(TEXT_DIM)),
            Span::styled("BUY ", Style::default().fg(GREEN).add_modifier(Modifier::BOLD)),
            Span::styled(" 24.100 SOL ", Style::default().fg(YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(" AlphA...ZZz0 ", Style::default().fg(TEXT)),
            Span::styled(" NEW WALLET", Style::default().fg(PURPLE).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled(" 14:22:51  ", Style::default().fg(TEXT_DIM)),
            Span::styled("SELL", Style::default().fg(RED).add_modifier(Modifier::BOLD)),
            Span::styled(" 15.200 SOL ", Style::default().fg(YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(" DUMP!...rEKt ", Style::default().fg(TEXT)),
            Span::styled(" TOP HOLDER", Style::default().fg(RED).add_modifier(Modifier::BOLD)),
        ]),
    ]).style(Style::default().bg(SURFACE3));
    f.render_widget(alerts, alert_inner);

    // Sorted trades (by size)
    let cols = Layout::horizontal([
        Constraint::Percentage(60), Constraint::Percentage(40),
    ]).split(rows[2]);

    let tb = Block::default()
        .title(Span::styled(" ALL TRADES (by size) ", Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL).border_style(Style::default().fg(DIM)).style(Style::default().bg(SURFACE)).padding(Padding::new(1, 1, 0, 0));
    let ti = tb.inner(cols[0]);
    f.render_widget(tb, cols[0]);

    let mut sorted: Vec<&FakeTrade> = TRADES.iter().collect();
    sorted.sort_by(|a, b| b.sol.partial_cmp(&a.sol).unwrap());

    let mut lines = Vec::new();
    lines.push(Line::from(vec![
        Span::styled("#  ", Style::default().fg(TEXT_DIM)),
        Span::styled("TIME      SIDE      AMOUNT        WALLET", Style::default().fg(TEXT_DIM)),
    ]));
    lines.push(Line::from(Span::styled("─".repeat(55), Style::default().fg(DIM))));
    for (i, t) in sorted.iter().take(14).enumerate() {
        let (side, sc) = if t.is_buy { ("BUY ", GREEN) } else { ("SELL", RED) };
        let rank_style = if i < 3 { Style::default().fg(YELLOW).add_modifier(Modifier::BOLD) } else { Style::default().fg(TEXT_DIM) };
        let amt_style = if t.sol >= 10.0 { Style::default().fg(YELLOW).add_modifier(Modifier::BOLD) }
            else if t.sol >= 5.0 { Style::default().fg(ORANGE).add_modifier(Modifier::BOLD) }
            else { Style::default().fg(TEXT) };
        lines.push(Line::from(vec![
            Span::styled(format!("{:<3}", i + 1), rank_style),
            Span::styled(format!("{} ", t.time), Style::default().fg(TEXT_DIM)),
            Span::styled(format!("{}  ", side), Style::default().fg(sc).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{:>8.3} SOL  ", t.sol), amt_style),
            Span::styled(t.wallet, Style::default().fg(TEXT_DIM)),
        ]));
    }
    f.render_widget(Paragraph::new(lines).style(Style::default().bg(SURFACE)), ti);

    // Right: top wallets
    let wb = Block::default()
        .title(Span::styled(" TOP WALLETS ", Style::default().fg(PURPLE).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL).border_style(Style::default().fg(DIM)).style(Style::default().bg(SURFACE)).padding(Padding::new(1, 1, 0, 0));
    let wi = wb.inner(cols[1]);
    f.render_widget(wb, cols[1]);

    let wallets = [
        ("AlphA...ZZz0", 24.10, 1, true),  ("DUMP!...rEKt", 15.20, 1, false),
        ("Sn1pR...Kill", 8.90, 1, false),   ("DiaM0...Hnd5", 6.78, 1, true),
        ("RugPu...LMAO", 5.67, 1, false),   ("ApeIn...NoW!", 4.56, 1, true),
        ("Whale...7pmQ", 3.20, 1, true),     ("chad0...xMax", 2.34, 1, true),
    ];
    let mut wlines = Vec::new();
    wlines.push(Line::from(vec![
        Span::styled("WALLET         SOL      SIDE", Style::default().fg(TEXT_DIM)),
    ]));
    wlines.push(Line::from(Span::styled("─".repeat(35), Style::default().fg(DIM))));
    for (w, sol, _, is_buy) in wallets {
        let c = if is_buy { GREEN } else { RED };
        let s = if is_buy { "BUY" } else { "SELL" };
        wlines.push(Line::from(vec![
            Span::styled(format!("{:<14} ", w), Style::default().fg(TEXT_DIM)),
            Span::styled(format!("{:>6.2}  ", sol), Style::default().fg(TEXT).add_modifier(Modifier::BOLD)),
            Span::styled(s, Style::default().fg(c).add_modifier(Modifier::BOLD)),
        ]));
    }
    f.render_widget(Paragraph::new(wlines).style(Style::default().bg(SURFACE)), wi);
}

// ═══════════════════════════════════════════════════════════
// STYLE 5: HEATMAP
// ═══════════════════════════════════════════════════════════
fn draw_style_heatmap(f: &mut Frame, area: Rect) {
    let rows = Layout::vertical([
        Constraint::Length(3), Constraint::Min(8), Constraint::Length(1),
    ]).split(area);
    fake_header(f, rows[0]);
    status_bar(f, rows[2]);

    let cols = Layout::horizontal([
        Constraint::Percentage(50), Constraint::Percentage(50),
    ]).split(rows[1]);

    // Volume heatmap
    let hb = Block::default()
        .title(Span::styled(" VOLUME HEATMAP (5m buckets) ", Style::default().fg(CYAN).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL).border_style(Style::default().fg(DIM)).style(Style::default().bg(SURFACE)).padding(Padding::new(1, 1, 0, 0));
    let hi = hb.inner(cols[0]);
    f.render_widget(hb, cols[0]);

    let heat_data: &[(&str, f64, f64)] = &[
        ("14:20", 8.2, 3.1), ("14:15", 12.4, 5.6), ("14:10", 3.1, 2.0),
        ("14:05", 18.9, 4.2), ("14:00", 6.7, 8.9), ("13:55", 2.1, 1.8),
        ("13:50", 14.3, 12.1), ("13:45", 5.5, 3.2), ("13:40", 1.2, 0.8),
        ("13:35", 22.1, 8.4), ("13:30", 9.8, 6.3), ("13:25", 4.2, 2.1),
    ];

    let mut hlines = Vec::new();
    hlines.push(Line::from(vec![
        Span::styled("TIME     ", Style::default().fg(TEXT_DIM)),
        Span::styled("BUYS                    SELLS", Style::default().fg(TEXT_DIM)),
    ]));
    for &(time, buy, sell) in heat_data {
        let max_v = 25.0;
        let bw_total = (hi.width as usize).saturating_sub(12) / 2;
        let bl = ((buy / max_v) * bw_total as f64).round() as usize;
        let sl = ((sell / max_v) * bw_total as f64).round() as usize;

        let buy_color = if buy > 15.0 { Color::Rgb(0, 255, 100) } else if buy > 8.0 { GREEN } else { Color::Rgb(30, 80, 50) };
        let sell_color = if sell > 10.0 { Color::Rgb(255, 50, 50) } else if sell > 5.0 { RED } else { Color::Rgb(80, 30, 30) };

        hlines.push(Line::from(vec![
            Span::styled(format!("{:<6}  ", time), Style::default().fg(TEXT_DIM)),
            Span::styled("█".repeat(bl.min(bw_total)), Style::default().fg(buy_color)),
            Span::styled("░".repeat(bw_total.saturating_sub(bl)), Style::default().fg(GRID)),
            Span::styled("│", Style::default().fg(DIM)),
            Span::styled("█".repeat(sl.min(bw_total)), Style::default().fg(sell_color)),
            Span::styled("░".repeat(bw_total.saturating_sub(sl)), Style::default().fg(GRID)),
        ]));
    }
    f.render_widget(Paragraph::new(hlines).style(Style::default().bg(SURFACE)), hi);

    // Right: chart + momentum
    let right_parts = Layout::vertical([
        Constraint::Min(6), Constraint::Length(6),
    ]).split(cols[1]);

    let cb = Block::default().title(Span::styled(" CHART ", Style::default().fg(CYAN).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL).border_style(Style::default().fg(DIM)).style(Style::default().bg(SURFACE));
    let ci = cb.inner(right_parts[0]);
    f.render_widget(cb, right_parts[0]);
    render_candles(f.buffer_mut(), ci, CANDLES);

    let mb = Block::default().borders(Borders::ALL).border_style(Style::default().fg(DIM)).style(Style::default().bg(SURFACE));
    let mi = mb.inner(right_parts[1]);
    f.render_widget(mb, right_parts[1]);
    render_momentum(f, mi);
}

// ═══════════════════════════════════════════════════════════
// STYLE 6: TAPE READER
// ═══════════════════════════════════════════════════════════
fn draw_style_tape_reader(f: &mut Frame, area: Rect) {
    let rows = Layout::vertical([
        Constraint::Length(2), Constraint::Min(8), Constraint::Length(3), Constraint::Length(1),
    ]).split(area);

    // Compact header
    let header = Paragraph::new(Line::from(vec![
        Span::styled(" AXIOM ", Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)),
        Span::styled("PUMP ", Style::default().fg(TEXT)),
        Span::styled("$128.4k ", Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)),
        Span::styled("▲12.3% ", Style::default().fg(GREEN)),
        Span::styled("│ ", Style::default().fg(DIM)),
        Span::styled("SOL $140.23 ", Style::default().fg(TEXT_DIM)),
        Span::styled("│ ", Style::default().fg(DIM)),
        Span::styled("TPS 3.8 ", Style::default().fg(CYAN)),
        Span::styled("│ ", Style::default().fg(DIM)),
        Span::styled("1,847 txns ", Style::default().fg(TEXT_DIM)),
        Span::styled("│ ", Style::default().fg(DIM)),
        Span::styled("312 wallets ", Style::default().fg(TEXT_DIM)),
        Span::styled("│ ", Style::default().fg(DIM)),
        Span::styled("STRONG BUY", Style::default().fg(GREEN).add_modifier(Modifier::BOLD)),
    ])).style(Style::default().bg(SURFACE2));
    f.render_widget(header, rows[0]);

    // Full-width tape
    let tape_block = Block::default()
        .title(Span::styled(" LIVE TAPE ", Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL).border_style(Style::default().fg(DIM)).style(Style::default().bg(SURFACE))
        .padding(Padding::new(1, 1, 0, 0));
    let tape_inner = tape_block.inner(rows[1]);
    f.render_widget(tape_block, rows[1]);

    let w = tape_inner.width as usize;
    let mut lines = Vec::new();
    lines.push(Line::from(vec![
        Span::styled(format!("{:<10}", "TIME"), Style::default().fg(TEXT_DIM)),
        Span::styled(format!("{:<6}", "SIDE"), Style::default().fg(TEXT_DIM)),
        Span::styled(format!("{:>12}", "AMOUNT"), Style::default().fg(TEXT_DIM)),
        Span::styled(format!("{:>10}", "PRICE"), Style::default().fg(TEXT_DIM)),
        Span::styled(format!("   {:<16}", "WALLET"), Style::default().fg(TEXT_DIM)),
        Span::styled(format!("{:>10}", "MCAP"), Style::default().fg(TEXT_DIM)),
        Span::styled(format!("   {}", "FLOW"), Style::default().fg(TEXT_DIM)),
    ]));
    lines.push(Line::from(Span::styled("─".repeat(w.min(100)), Style::default().fg(DIM))));

    let prices = [0.000183, 0.000182, 0.000180, 0.000188, 0.000187, 0.000179,
                   0.000181, 0.000180, 0.000173, 0.000174, 0.000176, 0.000175,
                   0.000178, 0.000177, 0.000176, 0.000179, 0.000178, 0.000169, 0.000172, 0.000171];

    for (i, t) in TRADES.iter().enumerate().take(tape_inner.height.saturating_sub(2) as usize) {
        let (side, sc) = if t.is_buy { ("BUY ", GREEN) } else { ("SELL", RED) };
        let amt_style = if t.sol >= 10.0 { Style::default().fg(YELLOW).add_modifier(Modifier::BOLD).bg(SURFACE3) }
            else if t.sol >= 5.0 { Style::default().fg(YELLOW).add_modifier(Modifier::BOLD) }
            else { Style::default().fg(TEXT) };
        let flow_len = (t.sol / 25.0 * 20.0).round() as usize;
        let flow_char = if t.is_buy { "▓" } else { "▓" };
        let flow_color = if t.is_buy { GREEN } else { RED };

        lines.push(Line::from(vec![
            Span::styled(format!("{:<10}", t.time), Style::default().fg(TEXT_DIM)),
            Span::styled(format!("{:<6}", side), Style::default().fg(sc).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{:>9.3} SOL", t.sol), amt_style),
            Span::styled(format!("{:>10.6}", prices[i % prices.len()]), Style::default().fg(TEXT_DIM)),
            Span::styled(format!("   {:<16}", t.wallet), Style::default().fg(TEXT_DIM)),
            Span::styled(format!("{:>10}", t.mcap), Style::default().fg(TEXT_DIM)),
            Span::styled(format!("   {}", flow_char.repeat(flow_len.min(20))), Style::default().fg(flow_color)),
        ]));
    }
    f.render_widget(Paragraph::new(lines).style(Style::default().bg(SURFACE)), tape_inner);

    // Volume bar at bottom
    let vol_block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(DIM)).style(Style::default().bg(SURFACE));
    let vol_inner = vol_block.inner(rows[2]);
    f.render_widget(vol_block, rows[2]);
    render_volume_bars(f, vol_inner);

    status_bar(f, rows[3]);
}

// ═══════════════════════════════════════════════════════════
// STYLE 7: SPLIT CHART
// ═══════════════════════════════════════════════════════════
fn draw_style_split_chart(f: &mut Frame, area: Rect) {
    let rows = Layout::vertical([
        Constraint::Length(3), Constraint::Min(8), Constraint::Length(1),
    ]).split(area);
    fake_header(f, rows[0]);
    status_bar(f, rows[2]);

    let cols = Layout::horizontal([
        Constraint::Percentage(50), Constraint::Percentage(50),
    ]).split(rows[1]);

    // Left: 5s chart + trades
    let left = Layout::vertical([
        Constraint::Percentage(50), Constraint::Percentage(50),
    ]).split(cols[0]);

    let cb1 = Block::default().title(Line::from(vec![
        Span::styled(" MC ", Style::default().fg(CYAN).add_modifier(Modifier::BOLD)),
        Span::styled("5s", Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)),
    ])).borders(Borders::ALL).border_style(Style::default().fg(DIM)).style(Style::default().bg(SURFACE));
    let ci1 = cb1.inner(left[0]);
    f.render_widget(cb1, left[0]);
    // Use fewer candles for 5s view
    render_candles(f.buffer_mut(), ci1, &CANDLES[8..]);

    render_trades(f, left[1], 8);

    // Right: 1m chart + stats
    let right = Layout::vertical([
        Constraint::Percentage(50), Constraint::Percentage(50),
    ]).split(cols[1]);

    let cb2 = Block::default().title(Line::from(vec![
        Span::styled(" MC ", Style::default().fg(CYAN).add_modifier(Modifier::BOLD)),
        Span::styled("1m", Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)),
    ])).borders(Borders::ALL).border_style(Style::default().fg(DIM)).style(Style::default().bg(SURFACE));
    let ci2 = cb2.inner(right[0]);
    f.render_widget(cb2, right[0]);
    render_candles(f.buffer_mut(), ci2, CANDLES);

    let sb = Block::default().title(Span::styled(" STATS ", Style::default().fg(YELLOW).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL).border_style(Style::default().fg(DIM)).style(Style::default().bg(SURFACE));
    let si = sb.inner(right[1]);
    f.render_widget(sb, right[1]);
    let sp = Layout::vertical([Constraint::Length(3), Constraint::Min(4)]).split(si);
    render_volume_bars(f, sp[0]);
    render_momentum(f, sp[1]);
}

// ═══════════════════════════════════════════════════════════
// STYLE 8: STRATEGY VIEW
// ═══════════════════════════════════════════════════════════
fn draw_style_strategy(f: &mut Frame, area: Rect) {
    let rows = Layout::vertical([
        Constraint::Length(3), Constraint::Length(5), Constraint::Min(8), Constraint::Length(1),
    ]).split(area);
    fake_header(f, rows[0]);
    status_bar(f, rows[3]);

    // Paper trading dashboard
    let ptb = Block::default()
        .title(Span::styled(" PAPER TRADING ", Style::default().fg(BLUE).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL).border_style(Style::default().fg(BLUE)).style(Style::default().bg(SURFACE3))
        .padding(Padding::new(1, 0, 0, 0));
    let pti = ptb.inner(rows[1]);
    f.render_widget(ptb, rows[1]);

    let pt = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Balance ", Style::default().fg(TEXT_DIM)),
            Span::styled("47.32 SOL", Style::default().fg(TEXT).add_modifier(Modifier::BOLD)),
            Span::styled("  │  Position ", Style::default().fg(DIM)),
            Span::styled("2,450,000 PUMP", Style::default().fg(CYAN).add_modifier(Modifier::BOLD)),
            Span::styled("  │  Entry ", Style::default().fg(DIM)),
            Span::styled("$121.5k MC", Style::default().fg(TEXT)),
            Span::styled("  │  Current ", Style::default().fg(DIM)),
            Span::styled("$128.4k MC", Style::default().fg(GREEN)),
        ]),
        Line::from(vec![
            Span::styled("P&L ", Style::default().fg(TEXT_DIM)),
            Span::styled("+2.84 SOL (+5.7%)", Style::default().fg(GREEN).add_modifier(Modifier::BOLD)),
            Span::styled("  │  Win Rate ", Style::default().fg(DIM)),
            Span::styled("68%", Style::default().fg(GREEN)),
            Span::styled(" (17W/8L)", Style::default().fg(TEXT_DIM)),
            Span::styled("  │  Sharpe ", Style::default().fg(DIM)),
            Span::styled("1.82", Style::default().fg(GREEN)),
            Span::styled("  │  Max DD ", Style::default().fg(DIM)),
            Span::styled("-12.3%", Style::default().fg(RED)),
        ]),
    ]).style(Style::default().bg(SURFACE3));
    f.render_widget(pt, pti);

    // Main: chart + signals
    let cols = Layout::horizontal([
        Constraint::Percentage(60), Constraint::Percentage(40),
    ]).split(rows[2]);

    let cb = Block::default().title(Line::from(vec![
        Span::styled(" CHART ", Style::default().fg(CYAN).add_modifier(Modifier::BOLD)),
        Span::styled("+ SIGNALS", Style::default().fg(BLUE)),
    ])).borders(Borders::ALL).border_style(Style::default().fg(DIM)).style(Style::default().bg(SURFACE));
    let ci = cb.inner(cols[0]);
    f.render_widget(cb, cols[0]);
    render_candles(f.buffer_mut(), ci, CANDLES);

    // Right: trade log + signals
    let right = Layout::vertical([
        Constraint::Percentage(60), Constraint::Percentage(40),
    ]).split(cols[1]);

    let sig_block = Block::default()
        .title(Span::styled(" SIGNALS ", Style::default().fg(BLUE).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL).border_style(Style::default().fg(DIM)).style(Style::default().bg(SURFACE))
        .padding(Padding::new(1, 1, 0, 0));
    let sig_inner = sig_block.inner(right[0]);
    f.render_widget(sig_block, right[0]);

    let signals = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("14:22:59 ", Style::default().fg(TEXT_DIM)),
            Span::styled("► ENTRY", Style::default().fg(GREEN).add_modifier(Modifier::BOLD)),
            Span::styled(" 2.5 SOL @ $121.5k MC", Style::default().fg(TEXT)),
        ]),
        Line::from(vec![
            Span::styled("         ", Style::default()),
            Span::styled("  Reason: ", Style::default().fg(TEXT_DIM)),
            Span::styled("Vol surge + buy pressure >75%", Style::default().fg(CYAN)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("14:23:01 ", Style::default().fg(TEXT_DIM)),
            Span::styled("▲ HOLD ", Style::default().fg(BLUE).add_modifier(Modifier::BOLD)),
            Span::styled(" +5.7% unrealized", Style::default().fg(GREEN)),
        ]),
        Line::from(vec![
            Span::styled("         ", Style::default()),
            Span::styled("  TP1: ", Style::default().fg(TEXT_DIM)),
            Span::styled("$150k (+16.8%)", Style::default().fg(GREEN)),
            Span::styled("  SL: ", Style::default().fg(TEXT_DIM)),
            Span::styled("$110k (-9.5%)", Style::default().fg(RED)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Momentum: ", Style::default().fg(TEXT_DIM)),
            Span::styled("████████████░░░░", Style::default().fg(GREEN)),
            Span::styled(" bullish", Style::default().fg(GREEN)),
        ]),
    ]).style(Style::default().bg(SURFACE));
    f.render_widget(signals, sig_inner);

    render_trades(f, right[1], 6);
}

// ═══════════════════════════════════════════════════════════
// STYLE 9: DARK TERMINAL (retro green)
// ═══════════════════════════════════════════════════════════
fn draw_style_dark_terminal(f: &mut Frame, area: Rect) {
    let g = Color::Rgb(0, 200, 0);
    let gd = Color::Rgb(0, 100, 0);
    let bg = Color::Rgb(5, 15, 5);
    let bg2 = Color::Rgb(10, 25, 10);

    f.render_widget(Block::default().style(Style::default().bg(bg)), area);

    let rows = Layout::vertical([
        Constraint::Length(4), Constraint::Min(8), Constraint::Length(1),
    ]).split(area);

    // Header
    let header = Paragraph::new(vec![
        Line::from(Span::styled(" ╔══ AXIOM TELEMETRY v0.1 ══╗", Style::default().fg(g).add_modifier(Modifier::BOLD))),
        Line::from(vec![
            Span::styled(" ║ TOKEN: ", Style::default().fg(gd)),
            Span::styled("PUMP", Style::default().fg(g).add_modifier(Modifier::BOLD)),
            Span::styled("  MCAP: ", Style::default().fg(gd)),
            Span::styled("$128.4k", Style::default().fg(g).add_modifier(Modifier::BOLD)),
            Span::styled("  SOL: ", Style::default().fg(gd)),
            Span::styled("$140.23", Style::default().fg(g)),
            Span::styled("  STATUS: ", Style::default().fg(gd)),
            Span::styled("ONLINE", Style::default().fg(g).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled(" ║ TXNS: ", Style::default().fg(gd)),
            Span::styled("1847", Style::default().fg(g)),
            Span::styled("  WALLETS: ", Style::default().fg(gd)),
            Span::styled("312", Style::default().fg(g)),
            Span::styled("  TPS: ", Style::default().fg(gd)),
            Span::styled("3.8", Style::default().fg(g)),
            Span::styled("  FEES: ", Style::default().fg(gd)),
            Span::styled("4.87 SOL", Style::default().fg(g)),
        ]),
    ]).style(Style::default().bg(bg));
    f.render_widget(header, rows[0]);

    // Main area
    let cols = Layout::horizontal([
        Constraint::Percentage(55), Constraint::Percentage(45),
    ]).split(rows[1]);

    // Trades in retro style
    let tb = Block::default()
        .title(Span::styled(" [TRADE LOG] ", Style::default().fg(g).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL).border_style(Style::default().fg(gd)).style(Style::default().bg(bg))
        .padding(Padding::new(1, 1, 0, 0));
    let ti = tb.inner(cols[0]);
    f.render_widget(tb, cols[0]);

    let mut lines = Vec::new();
    lines.push(Line::from(Span::styled(
        "TIME      DIR   AMOUNT       ADDR          MCAP", Style::default().fg(gd),
    )));
    lines.push(Line::from(Span::styled("─".repeat(55), Style::default().fg(gd))));

    for t in TRADES.iter().take(14) {
        let (dir, dc) = if t.is_buy {
            ("BUY ", Color::Rgb(0, 255, 0))
        } else {
            ("SELL", Color::Rgb(200, 0, 0))
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{} ", t.time), Style::default().fg(gd)),
            Span::styled(format!("{:<5}", dir), Style::default().fg(dc).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{:>8.3} SOL  ", t.sol), Style::default().fg(g)),
            Span::styled(format!("{:<14}", t.wallet), Style::default().fg(gd)),
            Span::styled(t.mcap, Style::default().fg(gd)),
        ]));
    }
    f.render_widget(Paragraph::new(lines).style(Style::default().bg(bg)), ti);

    // Right: ASCII art chart + stats
    let right = Layout::vertical([
        Constraint::Min(6), Constraint::Length(8),
    ]).split(cols[1]);

    let cb = Block::default()
        .title(Span::styled(" [PRICE] ", Style::default().fg(g).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL).border_style(Style::default().fg(gd)).style(Style::default().bg(bg));
    let ci = cb.inner(right[0]);
    f.render_widget(cb, right[0]);
    // Render candles but they'll use green/red which works fine on dark bg
    render_candles(f.buffer_mut(), ci, CANDLES);

    let sb = Block::default()
        .title(Span::styled(" [SYSTEM] ", Style::default().fg(g).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL).border_style(Style::default().fg(gd)).style(Style::default().bg(bg))
        .padding(Padding::new(1, 1, 0, 0));
    let si = sb.inner(right[1]);
    f.render_widget(sb, right[1]);

    let bw = si.width.saturating_sub(4) as usize;
    let bl = (bw as f64 * 0.78) as usize;
    let stats = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("BUY PRESSURE: ", Style::default().fg(gd)),
            Span::styled("78%", Style::default().fg(g).add_modifier(Modifier::BOLD)),
            Span::styled(" [STRONG]", Style::default().fg(Color::Rgb(0, 255, 0)).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("[", Style::default().fg(gd)),
            Span::styled("=".repeat(bl), Style::default().fg(g)),
            Span::styled("-".repeat(bw.saturating_sub(bl)), Style::default().fg(gd)),
            Span::styled("]", Style::default().fg(gd)),
        ]),
        Line::from(vec![
            Span::styled("VOL ACCEL: ", Style::default().fg(gd)),
            Span::styled("2.4x", Style::default().fg(g).add_modifier(Modifier::BOLD)),
            Span::styled(" [SURGING]", Style::default().fg(Color::Rgb(0, 255, 0))),
        ]),
        Line::from(vec![
            Span::styled("1m VOL: ", Style::default().fg(gd)),
            Span::styled("14.2 SOL", Style::default().fg(g)),
            Span::styled("  5m VOL: ", Style::default().fg(gd)),
            Span::styled("58.7 SOL", Style::default().fg(g)),
        ]),
        Line::from(vec![
            Span::styled("LARGEST: ", Style::default().fg(gd)),
            Span::styled("24.10 SOL", Style::default().fg(Color::Rgb(255, 255, 0)).add_modifier(Modifier::BOLD)),
            Span::styled(" [AlphA...ZZz0]", Style::default().fg(gd)),
        ]),
    ]).style(Style::default().bg(bg));
    f.render_widget(stats, si);

    // Status bar
    let status = Paragraph::new(Line::from(vec![
        Span::styled(" > ", Style::default().fg(g)),
        Span::styled("CONNECTED", Style::default().fg(g).add_modifier(Modifier::BOLD)),
        Span::styled(" | UPTIME 12:34 | ", Style::default().fg(gd)),
        Span::styled("q", Style::default().fg(g).add_modifier(Modifier::BOLD)),
        Span::styled("=quit ", Style::default().fg(gd)),
        Span::styled("t", Style::default().fg(g).add_modifier(Modifier::BOLD)),
        Span::styled("=timeframe ", Style::default().fg(gd)),
        Span::styled("_", Style::default().fg(g).add_modifier(Modifier::RAPID_BLINK)),
    ])).style(Style::default().bg(bg2));
    f.render_widget(status, rows[2]);
}
