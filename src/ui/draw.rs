use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph, Widget},
    Frame,
};

use super::console::App;

// ── Color palette ────────────────────────────────────────────
const ORANGE: Color = Color::Rgb(255, 165, 0);
const DIM: Color = Color::Rgb(90, 90, 90);
const SURFACE: Color = Color::Rgb(22, 22, 30);
const SURFACE_LIGHT: Color = Color::Rgb(30, 30, 42);
const TEXT: Color = Color::Rgb(200, 200, 210);
const TEXT_DIM: Color = Color::Rgb(120, 120, 140);
const GREEN: Color = Color::Rgb(80, 220, 120);
const RED: Color = Color::Rgb(255, 80, 80);
const CYAN: Color = Color::Rgb(80, 200, 255);
const YELLOW: Color = Color::Rgb(255, 220, 80);
const PURPLE: Color = Color::Rgb(160, 100, 255);
const GRID: Color = Color::Rgb(40, 40, 55);

// ── Main entry point ─────────────────────────────────────────

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    frame.render_widget(Block::default().style(Style::default().bg(SURFACE)), area);

    let outer = Layout::vertical([
        Constraint::Length(5),
        Constraint::Min(8),
        Constraint::Length(1),
    ])
    .split(area);

    draw_header(frame, app, outer[0]);
    draw_body(frame, app, outer[1]);
    draw_status_bar(frame, app, outer[2]);
}

// ── Header ───────────────────────────────────────────────────

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(DIM))
        .style(Style::default().bg(SURFACE));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let fdv_str = match app.fdv_usd {
        Some(fdv) => format_fdv(fdv),
        None => "...".to_string(),
    };

    // Row 1: branding + live indicator
    let dot = if app.connected { "●" } else { "○" };
    let dot_color = if app.connected { GREEN } else { RED };

    let row1 = Line::from(vec![
        Span::styled(" AXIOM", Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)),
        Span::styled(" TELEMETRY ", Style::default().fg(TEXT_DIM)),
        Span::styled(dot, Style::default().fg(dot_color)),
        Span::styled(
            if app.connected { " LIVE" } else { " OFFLINE" },
            Style::default().fg(dot_color).add_modifier(Modifier::BOLD),
        ),
    ]);

    // Row 2: token info with FULL contract address
    let row2 = Line::from(vec![
        Span::styled(format!(" {}", app.token_symbol), Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)),
        Span::styled(format!("  {}", app.token_name), Style::default().fg(TEXT)),
        Span::styled(format!("  {}", app.mint_address), Style::default().fg(TEXT_DIM)),
    ]);

    // Row 3: stats with ● separators + FDV % change
    let mut row3_spans = vec![
        Span::styled(" Market Cap ", Style::default().fg(TEXT_DIM)),
        Span::styled(&fdv_str, Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)),
    ];

    // 5-second FDV change arrow
    match app.fdv_change_pct() {
        Some(pct) if pct >= 0.0 => {
            row3_spans.push(Span::styled(
                format!(" ▲{:.1}%", pct),
                Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
            ));
        }
        Some(pct) => {
            row3_spans.push(Span::styled(
                format!(" ▼{:.1}%", pct.abs()),
                Style::default().fg(RED).add_modifier(Modifier::BOLD),
            ));
        }
        None => {}
    }

    row3_spans.extend(vec![
        Span::styled("  ●  ", Style::default().fg(DIM)),
        Span::styled("SOL ", Style::default().fg(TEXT_DIM)),
        Span::styled(format!("${:.2}", app.sol_price), Style::default().fg(TEXT)),
        Span::styled("  ●  ", Style::default().fg(DIM)),
        Span::styled("Txns ", Style::default().fg(TEXT_DIM)),
        Span::styled(format!("{}", app.tx_count), Style::default().fg(TEXT)),
        Span::styled("  ●  ", Style::default().fg(DIM)),
        Span::styled("Wallets ", Style::default().fg(TEXT_DIM)),
        Span::styled(format!("{}", app.unique_wallets.len()), Style::default().fg(TEXT)),
        Span::styled("  ●  ", Style::default().fg(DIM)),
        Span::styled("Fees ", Style::default().fg(TEXT_DIM)),
        Span::styled(format!("{:.4} SOL", app.total_fees_sol), Style::default().fg(ORANGE)),
    ]);

    let row3 = Line::from(row3_spans);
    let header = Paragraph::new(vec![row1, Line::from(""), row2, row3])
        .style(Style::default().bg(SURFACE));
    frame.render_widget(header, inner);
}

// ── Body: switches between Normal (Bloomberg) and Alert view ──

fn draw_body(frame: &mut Frame, app: &App, area: Rect) {
    use super::console::ViewMode;
    match app.view_mode {
        ViewMode::Normal => draw_body_normal(frame, app, area),
        ViewMode::Alert => draw_body_alert(frame, app, area),
    }
}

fn draw_body_normal(frame: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::vertical([
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ])
    .split(area);

    let top_cols = Layout::horizontal([
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ])
    .split(rows[0]);

    let bot_cols = Layout::horizontal([
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ])
    .split(rows[1]);

    draw_chart_panel(frame, app, top_cols[0]);
    draw_trades_panel(frame, app, top_cols[1]);
    draw_orderflow_panel(frame, app, bot_cols[0]);
    draw_volume_stats_panel(frame, app, bot_cols[1]);
}

fn draw_body_alert(frame: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::vertical([
        Constraint::Percentage(70),
        Constraint::Percentage(30),
    ])
    .split(area);

    // Top: chart (75%) + quick facts (25%)
    let top_cols = Layout::horizontal([
        Constraint::Percentage(75),
        Constraint::Percentage(25),
    ])
    .split(rows[0]);

    draw_chart_panel(frame, app, top_cols[0]);
    draw_quick_facts(frame, app, top_cols[1]);

    // Bottom: full-width alerts
    draw_alerts_panel(frame, app, rows[1]);
}

// ── Top-Left: Chart ──────────────────────────────────────────

fn draw_chart_panel(frame: &mut Frame, app: &App, area: Rect) {
    let tf_label = app.chart_timeframe.label();
    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" CHART ", Style::default().fg(CYAN).add_modifier(Modifier::BOLD)),
            Span::styled(tf_label, Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)),
            Span::styled("  t", Style::default().fg(TEXT_DIM)),
        ]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(DIM))
        .style(Style::default().bg(SURFACE));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let (candles, global_low, global_high, total_slots) = app.get_ohlc_data();

    if candles.is_empty() {
        let msg = Paragraph::new(Line::from(Span::styled(
            " Waiting for data...",
            Style::default().fg(TEXT_DIM),
        )))
        .style(Style::default().bg(SURFACE));
        frame.render_widget(msg, inner);
        return;
    }

    let widget = CandlestickChart {
        candles: &candles,
        global_low,
        global_high,
        total_slots,
    };
    frame.render_widget(widget, inner);
}

// ── Top-Right: Trades ────────────────────────────────────────

fn draw_trades_panel(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(Span::styled(" TRADES ", Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(DIM))
        .style(Style::default().bg(SURFACE))
        .padding(Padding::new(1, 1, 0, 0));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.trades.is_empty() {
        let msg = Paragraph::new(Line::from(Span::styled(
            "Waiting for trades...",
            Style::default().fg(TEXT_DIM),
        )))
        .style(Style::default().bg(SURFACE));
        frame.render_widget(msg, inner);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    let w = inner.width as usize;

    let col_time = 9;
    let col_side = 6;
    let col_amount = 11;
    let col_mcap = 8;
    let col_wallet = w.saturating_sub(col_time + col_side + col_amount + col_mcap);

    let center = |s: &str, width: usize| -> String {
        let slen = s.len().min(width);
        let pad_left = (width.saturating_sub(slen)) / 2;
        let pad_right = width.saturating_sub(slen + pad_left);
        format!("{}{}{}", " ".repeat(pad_left), &s[..slen], " ".repeat(pad_right))
    };

    lines.push(Line::from(vec![
        Span::styled(format!("{:<col_time$}", "TIME"), Style::default().fg(TEXT_DIM)),
        Span::styled(format!("{:<col_side$}", "SIDE"), Style::default().fg(TEXT_DIM)),
        Span::styled(format!("{:>col_amount$}", "AMOUNT"), Style::default().fg(TEXT_DIM)),
        Span::styled(center("WALLET", col_wallet), Style::default().fg(TEXT_DIM)),
        Span::styled(format!("{:>col_mcap$}", "MCAP"), Style::default().fg(TEXT_DIM)),
    ]));
    lines.push(Line::from(Span::styled("─".repeat(w), Style::default().fg(DIM))));

    let max_rows = inner.height.saturating_sub(2) as usize;
    for t in app.trades.iter().take(max_rows) {
        let (side_text, side_style) = if t.is_buy {
            ("BUY", Style::default().fg(GREEN).add_modifier(Modifier::BOLD))
        } else {
            ("SELL", Style::default().fg(RED).add_modifier(Modifier::BOLD))
        };

        // SOL amount in orange, whale trades in yellow
        let amt_color = if t.sol_amount >= 5.0 { YELLOW } else { ORANGE };
        let amount_num = format!("{:.3}", t.sol_amount);
        let mcap_str = match t.mcap_usd {
            Some(m) => format_fdv_compact(m),
            None => "---".to_string(),
        };

        lines.push(Line::from(vec![
            Span::styled(format!("{:<col_time$}", t.timestamp), Style::default().fg(TEXT_DIM)),
            Span::styled(format!("{:<col_side$}", side_text), side_style),
            Span::styled(format!("{:>7}", amount_num), Style::default().fg(amt_color).add_modifier(Modifier::BOLD)),
            Span::styled(" SOL", Style::default().fg(ORANGE)),
            Span::styled(center(&t.wallet_short, col_wallet.saturating_sub(4)), Style::default().fg(TEXT_DIM)),
            Span::styled(format!("{:>col_mcap$}", mcap_str), Style::default().fg(TEXT_DIM)),
        ]));
    }

    frame.render_widget(Paragraph::new(lines).style(Style::default().bg(SURFACE)), inner);
}

// ── Bottom-Left: Order Flow ──────────────────────────────────

fn draw_orderflow_panel(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(Span::styled(
            " ORDER FLOW ",
            Style::default().fg(PURPLE).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(DIM))
        .style(Style::default().bg(SURFACE))
        .padding(Padding::new(1, 1, 0, 0));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    // Collect trades with mcap data
    let mut trades_with_mcap: Vec<_> = app
        .trades
        .iter()
        .filter_map(|t| t.mcap_usd.map(|m| (m, t.sol_amount, t.is_buy)))
        .collect();

    if trades_with_mcap.is_empty() {
        lines.push(Line::from(Span::styled(
            "Waiting for data...",
            Style::default().fg(TEXT_DIM),
        )));
        frame.render_widget(
            Paragraph::new(lines).style(Style::default().bg(SURFACE)),
            inner,
        );
        return;
    }

    // Outlier rejection: use median mcap, clip to 3x range
    let mut mcap_vals: Vec<f64> = trades_with_mcap.iter().map(|(m, _, _)| *m).collect();
    mcap_vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_mcap = mcap_vals[mcap_vals.len() / 2];
    let clip_lo = median_mcap / 3.0;
    let clip_hi = median_mcap * 3.0;
    trades_with_mcap.retain(|(m, _, _)| *m >= clip_lo && *m <= clip_hi);

    if trades_with_mcap.is_empty() {
        lines.push(Line::from(Span::styled(
            "Waiting for data...",
            Style::default().fg(TEXT_DIM),
        )));
        frame.render_widget(
            Paragraph::new(lines).style(Style::default().bg(SURFACE)),
            inner,
        );
        return;
    }

    lines.push(Line::from(vec![
        Span::styled("PRICE LVL     ", Style::default().fg(TEXT_DIM)),
        Span::styled("BUYS            SELLS", Style::default().fg(TEXT_DIM)),
    ]));

    // Find mcap range (outliers already clipped)
    let min_mcap = trades_with_mcap
        .iter()
        .map(|(m, _, _)| *m)
        .fold(f64::INFINITY, f64::min);
    let max_mcap = trades_with_mcap
        .iter()
        .map(|(m, _, _)| *m)
        .fold(f64::NEG_INFINITY, f64::max);
    let range = (max_mcap - min_mcap).max(1.0);

    let num_levels = (inner.height as usize).saturating_sub(1).min(12).max(2);
    let bucket_size = range / num_levels as f64;

    // Aggregate by bucket
    let mut buy_vols = vec![0.0f64; num_levels];
    let mut sell_vols = vec![0.0f64; num_levels];

    for &(mcap, sol, is_buy) in &trades_with_mcap {
        let idx = ((mcap - min_mcap) / bucket_size).floor() as usize;
        let idx = idx.min(num_levels - 1);
        if is_buy {
            buy_vols[idx] += sol;
        } else {
            sell_vols[idx] += sol;
        }
    }

    let max_vol = buy_vols
        .iter()
        .chain(sell_vols.iter())
        .cloned()
        .fold(0.0f64, f64::max)
        .max(0.001);
    let bar_max = (inner.width as usize).saturating_sub(16) / 2;

    // Render from highest price level to lowest
    for i in (0..num_levels).rev() {
        let level_mcap = min_mcap + (i as f64 + 0.5) * bucket_size;
        let label = format_fdv_compact(level_mcap);
        let bl = ((buy_vols[i] / max_vol) * bar_max as f64).round() as usize;
        let sl = ((sell_vols[i] / max_vol) * bar_max as f64).round() as usize;

        let buy_color = if buy_vols[i] > max_vol * 0.6 {
            Color::Rgb(0, 255, 100)
        } else {
            GREEN
        };
        let sell_color = if sell_vols[i] > max_vol * 0.6 {
            Color::Rgb(255, 50, 50)
        } else {
            RED
        };

        lines.push(Line::from(vec![
            Span::styled(format!("{:<8} ", label), Style::default().fg(TEXT_DIM)),
            Span::styled(
                "█".repeat(bl.min(bar_max)),
                Style::default().fg(buy_color),
            ),
            Span::styled(
                "░".repeat(bar_max.saturating_sub(bl)),
                Style::default().fg(GRID),
            ),
            Span::styled("│", Style::default().fg(DIM)),
            Span::styled(
                "█".repeat(sl.min(bar_max)),
                Style::default().fg(sell_color),
            ),
            Span::styled(
                "░".repeat(bar_max.saturating_sub(sl)),
                Style::default().fg(GRID),
            ),
        ]));
    }

    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(SURFACE)),
        inner,
    );
}

// ── Bottom-Right: Volume + Momentum ──────────────────────────

fn draw_volume_stats_panel(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(Span::styled(
            " STATS ",
            Style::default().fg(ORANGE).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(DIM))
        .style(Style::default().bg(SURFACE));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let sections = Layout::vertical([
        Constraint::Length(7),
        Constraint::Min(4),
    ])
    .split(inner);

    draw_volume_section(frame, app, sections[0]);
    draw_momentum_section(frame, app, sections[1]);
}

fn draw_volume_section(frame: &mut Frame, app: &App, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();
    let bar_w = area.width.saturating_sub(4) as usize;

    lines.push(Line::from(Span::styled(
        " VOLUME",
        Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
    )));

    // 4 timeframes with % change arrows
    for &(label, secs) in &[("5s", 5u64), ("30s", 30), ("1m", 60), ("5m", 300)] {
        let pct = app.volume.volume_change_pct(app.now_epoch, secs);
        let (arrow, color) = if pct.abs() < 0.01 {
            ("  ---".to_string(), TEXT_DIM)
        } else if pct >= 0.0 {
            (format!(" ▲ {:.1}%", pct), GREEN)
        } else {
            (format!(" ▼ {:.1}%", pct.abs()), RED)
        };
        lines.push(Line::from(vec![
            Span::styled(format!(" {:<4}", label), Style::default().fg(TEXT_DIM)),
            Span::styled(arrow, Style::default().fg(color).add_modifier(Modifier::BOLD)),
        ]));
    }

    // Buy/sell bar for 1m
    let (buy_1m, sell_1m) = app.volume.volume_in_window(app.now_epoch, 60);
    lines.push(make_volume_bar(buy_1m, sell_1m, bar_w));

    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(SURFACE)),
        area,
    );
}

fn draw_momentum_section(frame: &mut Frame, app: &App, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();
    let bar_w = area.width.saturating_sub(4) as usize;

    lines.push(Line::from(Span::styled(
        " MOMENTUM",
        Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
    )));

    // Buy pressure
    let pressure = app.volume.buy_pressure(app.now_epoch, 30);
    let pressure_label = if pressure > 0.65 {
        "STRONG BUY"
    } else if pressure > 0.55 {
        "BUY"
    } else if pressure < 0.35 {
        "STRONG SELL"
    } else if pressure < 0.45 {
        "SELL"
    } else {
        "NEUTRAL"
    };
    let pressure_color = if pressure > 0.55 {
        GREEN
    } else if pressure < 0.45 {
        RED
    } else {
        TEXT_DIM
    };

    lines.push(Line::from(vec![
        Span::styled(" Pressure ", Style::default().fg(TEXT_DIM)),
        Span::styled(
            pressure_label,
            Style::default()
                .fg(pressure_color)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(make_pressure_bar(pressure, bar_w));

    // Volume acceleration
    let accel = app.volume.volume_acceleration(app.now_epoch, 30);
    let accel_label = if accel > 2.0 {
        "SURGING"
    } else if accel > 1.3 {
        "RISING"
    } else if accel < 0.5 {
        "FADING"
    } else if accel < 0.8 {
        "SLOWING"
    } else {
        "STEADY"
    };
    let accel_color = if accel > 1.3 {
        GREEN
    } else if accel < 0.8 {
        RED
    } else {
        TEXT_DIM
    };

    lines.push(Line::from(vec![
        Span::styled(" Vol Δ    ", Style::default().fg(TEXT_DIM)),
        Span::styled(
            format!("{} ({:.1}x)", accel_label, accel),
            Style::default()
                .fg(accel_color)
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    // TPS + trade counts + largest
    let tps = app.volume.tps(app.now_epoch, 60);
    let (buys, sells) = app.volume.count_in_window(app.now_epoch, 300);
    lines.push(Line::from(vec![
        Span::styled(" TPS ", Style::default().fg(TEXT_DIM)),
        Span::styled(format!("{:.1}", tps), Style::default().fg(TEXT)),
        Span::styled("  ●  ", Style::default().fg(DIM)),
        Span::styled("5m ", Style::default().fg(TEXT_DIM)),
        Span::styled(format!("{}", buys), Style::default().fg(GREEN)),
        Span::styled("B ", Style::default().fg(TEXT_DIM)),
        Span::styled(format!("{}", sells), Style::default().fg(RED)),
        Span::styled("S", Style::default().fg(TEXT_DIM)),
        Span::styled("  ●  ", Style::default().fg(DIM)),
        Span::styled("Lg ", Style::default().fg(TEXT_DIM)),
        Span::styled(
            format!("{:.2}", app.largest_trade_sol),
            Style::default().fg(if app.largest_trade_sol >= 5.0 {
                YELLOW
            } else {
                TEXT
            }),
        ),
    ]));

    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(SURFACE)),
        area,
    );
}

// ── Quick Facts (Alert view, top-right) ──────────────────────

fn draw_quick_facts(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(Span::styled(
            " QUICK FACTS ",
            Style::default().fg(ORANGE).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(DIM))
        .style(Style::default().bg(SURFACE))
        .padding(Padding::new(1, 1, 0, 0));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    // Volume % changes
    lines.push(Line::from(Span::styled(
        "VOLUME",
        Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
    )));
    for &(label, secs) in &[("5s", 5u64), ("30s", 30), ("1m", 60), ("5m", 300)] {
        let pct = app.volume.volume_change_pct(app.now_epoch, secs);
        let (arrow, color) = if pct.abs() < 0.01 {
            ("  ---".to_string(), TEXT_DIM)
        } else if pct >= 0.0 {
            (format!(" ▲{:.0}%", pct), GREEN)
        } else {
            (format!(" ▼{:.0}%", pct.abs()), RED)
        };
        lines.push(Line::from(vec![
            Span::styled(format!(" {:<4}", label), Style::default().fg(TEXT_DIM)),
            Span::styled(arrow, Style::default().fg(color).add_modifier(Modifier::BOLD)),
        ]));
    }

    lines.push(Line::from(""));

    // Pressure
    let pressure = app.volume.buy_pressure(app.now_epoch, 30);
    let pressure_label = if pressure > 0.65 { "STRONG BUY" }
        else if pressure > 0.55 { "BUY" }
        else if pressure < 0.35 { "STRONG SELL" }
        else if pressure < 0.45 { "SELL" }
        else { "NEUTRAL" };
    let pc = if pressure > 0.55 { GREEN } else if pressure < 0.45 { RED } else { TEXT_DIM };
    lines.push(Line::from(vec![
        Span::styled(" Pressure ", Style::default().fg(TEXT_DIM)),
        Span::styled(pressure_label, Style::default().fg(pc).add_modifier(Modifier::BOLD)),
    ]));

    // TPS
    let tps = app.volume.tps(app.now_epoch, 60);
    lines.push(Line::from(vec![
        Span::styled(" TPS ", Style::default().fg(TEXT_DIM)),
        Span::styled(format!("{:.1}", tps), Style::default().fg(TEXT)),
    ]));

    // Vol acceleration
    let accel = app.volume.volume_acceleration(app.now_epoch, 30);
    let accel_label = if accel > 2.0 { "SURGING" }
        else if accel > 1.3 { "RISING" }
        else if accel < 0.5 { "FADING" }
        else if accel < 0.8 { "SLOWING" }
        else { "STEADY" };
    let ac = if accel > 1.3 { GREEN } else if accel < 0.8 { RED } else { TEXT_DIM };
    lines.push(Line::from(vec![
        Span::styled(" Vol Δ ", Style::default().fg(TEXT_DIM)),
        Span::styled(format!("{} {:.1}x", accel_label, accel), Style::default().fg(ac).add_modifier(Modifier::BOLD)),
    ]));

    lines.push(Line::from(""));

    // Largest trade
    lines.push(Line::from(vec![
        Span::styled(" Largest ", Style::default().fg(TEXT_DIM)),
        Span::styled(
            format!("{:.2} SOL", app.largest_trade_sol),
            Style::default().fg(if app.largest_trade_sol >= 5.0 { YELLOW } else { TEXT }),
        ),
    ]));

    // Trade counts
    let (b5, s5) = app.volume.count_in_window(app.now_epoch, 300);
    lines.push(Line::from(vec![
        Span::styled(" 5m ", Style::default().fg(TEXT_DIM)),
        Span::styled(format!("{}", b5), Style::default().fg(GREEN)),
        Span::styled("B ", Style::default().fg(TEXT_DIM)),
        Span::styled(format!("{}", s5), Style::default().fg(RED)),
        Span::styled("S", Style::default().fg(TEXT_DIM)),
    ]));

    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(SURFACE)),
        inner,
    );
}

// ── Alerts panel (Alert view, bottom) ────────────────────────

fn draw_alerts_panel(frame: &mut Frame, app: &App, area: Rect) {
    use super::console::AlertKind;

    let block = Block::default()
        .title(Span::styled(
            " SIGNALS ",
            Style::default().fg(YELLOW).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(DIM))
        .style(Style::default().bg(SURFACE))
        .padding(Padding::new(1, 1, 0, 0));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let alerts = &app.alert_engine.alerts;
    if alerts.is_empty() {
        let msg = Paragraph::new(Line::from(Span::styled(
            "Monitoring for signals...",
            Style::default().fg(TEXT_DIM),
        )))
        .style(Style::default().bg(SURFACE));
        frame.render_widget(msg, inner);
        return;
    }

    let max_rows = inner.height as usize;
    let mut lines: Vec<Line> = Vec::new();

    for alert in alerts.iter().take(max_rows) {
        let color = match alert.kind {
            AlertKind::WhaleBuy | AlertKind::BuyWall | AlertKind::McapBreakout => GREEN,
            AlertKind::WhaleSell | AlertKind::SellWall | AlertKind::McapDump => RED,
            AlertKind::VolumeSurge => CYAN,
            AlertKind::VolumeDying => TEXT_DIM,
            AlertKind::NewAth => YELLOW,
            AlertKind::FreshWallets => PURPLE,
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{} ", alert.timestamp), Style::default().fg(TEXT_DIM)),
            Span::styled(
                format!("{:<12}", alert.kind.label()),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(&alert.message, Style::default().fg(TEXT)),
        ]));
    }

    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(SURFACE)),
        inner,
    );
}

// ── Status bar ───────────────────────────────────────────────

fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let last_update_str = match app.secs_since_update() {
        Some(0) | Some(1) => "just now".to_string(),
        Some(s) if s < 60 => format!("{}s ago", s),
        Some(s) => format!("{}m ago", s / 60),
        None => "never".to_string(),
    };

    let line = Line::from(vec![
        Span::styled(" ", Style::default()),
        Span::styled(
            if app.connected { "●" } else { "○" },
            Style::default().fg(if app.connected { GREEN } else { RED }),
        ),
        Span::styled(
            if app.connected {
                " Connected"
            } else {
                " Disconnected"
            },
            Style::default().fg(TEXT_DIM),
        ),
        Span::styled("  │  ", Style::default().fg(DIM)),
        Span::styled("Last update ", Style::default().fg(TEXT_DIM)),
        Span::styled(&last_update_str, Style::default().fg(TEXT)),
        Span::styled("  │  ", Style::default().fg(DIM)),
        Span::styled("Uptime ", Style::default().fg(TEXT_DIM)),
        Span::styled(app.uptime(), Style::default().fg(TEXT)),
        Span::styled("  │  ", Style::default().fg(DIM)),
        Span::styled("q", Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)),
        Span::styled(" quit  ", Style::default().fg(TEXT_DIM)),
        Span::styled("t", Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)),
        Span::styled(" timeframe  ", Style::default().fg(TEXT_DIM)),
        Span::styled("F2", Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)),
        Span::styled(
            if matches!(app.view_mode, super::console::ViewMode::Alert) { " normal" } else { " alerts" },
            Style::default().fg(TEXT_DIM),
        ),
    ]);

    let bar = Paragraph::new(line).style(Style::default().bg(SURFACE_LIGHT));
    frame.render_widget(bar, area);
}

// ── Shared bar helpers ───────────────────────────────────────

fn make_volume_bar(buy: f64, sell: f64, width: usize) -> Line<'static> {
    let bar_width = width.saturating_sub(2);
    let total = buy + sell;
    if total == 0.0 {
        return Line::from(Span::styled(
            format!(" {}", "░".repeat(bar_width)),
            Style::default().fg(DIM),
        ));
    }

    let buy_len = ((buy / total) * bar_width as f64).round() as usize;
    let sell_len = bar_width.saturating_sub(buy_len);
    let buy_pct = (buy / total * 100.0) as u32;
    let sell_pct = 100u32.saturating_sub(buy_pct);

    Line::from(vec![
        Span::styled(" ", Style::default()),
        Span::styled("█".repeat(buy_len), Style::default().fg(GREEN)),
        Span::styled("█".repeat(sell_len), Style::default().fg(RED)),
        Span::styled(
            format!(" {}%/{}%", buy_pct, sell_pct),
            Style::default().fg(TEXT_DIM),
        ),
    ])
}

fn make_pressure_bar(pressure: f64, width: usize) -> Line<'static> {
    let bar_width = width.saturating_sub(2);
    if bar_width < 4 {
        return Line::from("");
    }
    let buy_len = ((pressure * bar_width as f64).round() as usize).min(bar_width);
    let sell_len = bar_width.saturating_sub(buy_len);

    Line::from(vec![
        Span::styled(" ", Style::default()),
        Span::styled("█".repeat(buy_len), Style::default().fg(GREEN)),
        Span::styled("█".repeat(sell_len), Style::default().fg(RED)),
        Span::styled(
            format!(" {:.0}%B", pressure * 100.0),
            Style::default().fg(TEXT_DIM),
        ),
    ])
}

// ── Candlestick chart widget ─────────────────────────────────

const Y_AXIS_WIDTH: u16 = 9;
const CANDLE_GREEN: Color = Color::Rgb(38, 166, 91);
const CANDLE_RED: Color = Color::Rgb(234, 57, 67);

struct CandlestickChart<'a> {
    candles: &'a [(usize, super::console::Candle)],
    global_low: f64,
    global_high: f64,
    total_slots: usize,
}

impl<'a> Widget for CandlestickChart<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 3 || area.width < Y_AXIS_WIDTH + 4 {
            return;
        }

        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf[(x, y)].set_style(Style::default().bg(SURFACE));
            }
        }

        let chart_w = area.width.saturating_sub(Y_AXIS_WIDTH) as usize;
        let chart_h = area.height as usize;
        let axis_x = area.x + chart_w as u16;

        let range = self.global_high - self.global_low;
        let pad = if range < f64::EPSILON {
            self.global_high * 0.1
        } else {
            range * 0.1
        };
        let y_min = (self.global_low - pad).max(0.0);
        let y_max = self.global_high + pad;
        let y_range = (y_max - y_min).max(f64::EPSILON);

        // Grid lines
        let grid_count = 4usize;
        for i in 0..=grid_count {
            let row =
                area.y + (i as u16 * (area.height.saturating_sub(1))) / grid_count as u16;
            if row >= area.y + area.height {
                continue;
            }
            for x in area.x..area.x + chart_w as u16 {
                if (x - area.x) % 3 == 0 {
                    buf[(x, row)].set_char('·');
                    buf[(x, row)].set_fg(GRID);
                }
            }
            let val = y_max - (i as f64 / grid_count as f64) * y_range;
            let label = format_fdv_compact(val);
            let label_x = axis_x + 1;
            for (ci, ch) in label.chars().enumerate() {
                let px = label_x + ci as u16;
                if px < area.x + area.width {
                    buf[(px, row)].set_char(ch);
                    buf[(px, row)].set_fg(TEXT_DIM);
                }
            }
        }

        // Y-axis separator
        for y in area.y..area.y + area.height {
            if axis_x < area.x + area.width {
                buf[(axis_x, y)].set_char('│');
                buf[(axis_x, y)].set_fg(DIM);
            }
        }

        let val_to_row = |val: f64| -> usize {
            let frac = (val - y_min) / y_range;
            ((1.0 - frac) * (chart_h as f64 - 1.0)).round() as usize
        };

        let n = self.total_slots.max(1);
        if self.candles.is_empty() {
            return;
        }

        // Pitch based on total_slots (always 60), not candle count
        // This gives each slot a fixed position — chart fills left to right
        let candle_pitch = chart_w as f64 / n as f64;
        let raw_body = (candle_pitch - 0.5).floor() as usize;
        let body_width = match raw_body.max(1).min(7) {
            w if w > 1 && w % 2 == 0 => w - 1,
            w => w,
        };

        for &(slot_idx, ref candle) in self.candles.iter() {
            if !candle.has_data {
                continue;
            }

            let center_x = area.x + ((slot_idx as f64 + 0.5) * candle_pitch) as u16;
            if center_x >= area.x + chart_w as u16 {
                break;
            }

            let is_green = candle.close >= candle.open;
            let color = if is_green { CANDLE_GREEN } else { CANDLE_RED };

            let row_high = val_to_row(candle.high);
            let row_low = val_to_row(candle.low);
            let row_open = val_to_row(candle.open);
            let row_close = val_to_row(candle.close);
            let body_top = row_open.min(row_close);
            let body_bot = row_open.max(row_close);

            // Wick
            for row in row_high..=row_low {
                let py = area.y + row as u16;
                if py < area.y + area.height && center_x < area.x + area.width {
                    buf[(center_x, py)].set_char('│');
                    buf[(center_x, py)].set_fg(color);
                }
            }

            // Body
            let half_w = body_width / 2;
            let bstart = center_x.saturating_sub(half_w as u16);
            let bend = (center_x + half_w as u16 + 1).min(area.x + chart_w as u16);

            if body_top == body_bot {
                let py = area.y + body_top as u16;
                if py < area.y + area.height {
                    let doji_half = (body_width / 2).min(1);
                    let ds = center_x.saturating_sub(doji_half as u16);
                    let de = (center_x + doji_half as u16 + 1).min(area.x + chart_w as u16);
                    for px in ds..de {
                        if px >= area.x && px < area.x + chart_w as u16 {
                            buf[(px, py)].set_char('─');
                            buf[(px, py)].set_fg(color);
                            buf[(px, py)].set_bg(SURFACE);
                        }
                    }
                }
            } else {
                for row in body_top..=body_bot {
                    let py = area.y + row as u16;
                    if py >= area.y + area.height {
                        continue;
                    }
                    for px in bstart..bend {
                        if px >= area.x && px < area.x + chart_w as u16 {
                            buf[(px, py)].set_char('█');
                            buf[(px, py)].set_fg(color);
                            buf[(px, py)].set_bg(SURFACE);
                        }
                    }
                }
            }
        }

        // Current price marker
        if let Some(&(_, ref last)) = self.candles.last() {
            let row = val_to_row(last.close);
            let py = area.y + row as u16;
            let marker_color = if last.close >= last.open {
                CANDLE_GREEN
            } else {
                CANDLE_RED
            };

            let label = format_fdv_compact(last.close);
            let label_x = axis_x + 1;
            if py < area.y + area.height {
                buf[(axis_x, py)].set_char('◄');
                buf[(axis_x, py)].set_fg(marker_color);
                for (ci, ch) in label.chars().enumerate() {
                    let px = label_x + ci as u16;
                    if px < area.x + area.width {
                        buf[(px, py)].set_char(ch);
                        buf[(px, py)].set_fg(Color::White);
                        buf[(px, py)].set_bg(marker_color);
                    }
                }
            }
        }
    }
}

// ── Formatting helpers ───────────────────────────────────────

fn format_fdv(fdv: f64) -> String {
    if fdv >= 1_000_000_000.0 {
        format!("${:.1}B", fdv / 1_000_000_000.0)
    } else if fdv >= 1_000_000.0 {
        format!("${:.1}M", fdv / 1_000_000.0)
    } else if fdv >= 1_000.0 {
        format!("${:.1}k", fdv / 1_000.0)
    } else {
        format!("${:.2}", fdv)
    }
}

fn format_fdv_compact(fdv: f64) -> String {
    if fdv >= 1_000_000_000.0 {
        format!("${:.2}B", fdv / 1_000_000_000.0)
    } else if fdv >= 1_000_000.0 {
        format!("${:.2}M", fdv / 1_000_000.0)
    } else if fdv >= 1_000.0 {
        format!("${:.2}k", fdv / 1_000.0)
    } else if fdv >= 1.0 {
        format!("${:.2}", fdv)
    } else {
        format!("${:.4}", fdv)
    }
}
