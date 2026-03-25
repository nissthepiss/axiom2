use std::collections::{HashMap, VecDeque};
use std::time::Instant;

const MAX_TRADES: usize = 200;
const MAX_PRICE_POINTS: usize = 60;
const MAX_ALERTS: usize = 50;

// ── Alert system ─────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlertKind {
    WhaleBuy,
    WhaleSell,
    VolumeSurge,
    VolumeDying,
    BuyWall,
    SellWall,
    McapBreakout,
    McapDump,
    NewAth,
    FreshWallets,
}

impl AlertKind {
    pub fn cooldown_secs(&self) -> u64 {
        match self {
            AlertKind::WhaleBuy | AlertKind::WhaleSell => 5,
            AlertKind::VolumeSurge => 8,
            AlertKind::VolumeDying => 10,
            AlertKind::BuyWall | AlertKind::SellWall => 8,
            AlertKind::McapBreakout | AlertKind::McapDump => 6,
            AlertKind::NewAth => 5,
            AlertKind::FreshWallets => 5,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            AlertKind::WhaleBuy => "WHALE BUY",
            AlertKind::WhaleSell => "WHALE SELL",
            AlertKind::VolumeSurge => "VOL SURGE",
            AlertKind::VolumeDying => "VOL DYING",
            AlertKind::BuyWall => "BUY WALL",
            AlertKind::SellWall => "SELL WALL",
            AlertKind::McapBreakout => "BREAKOUT",
            AlertKind::McapDump => "DUMP",
            AlertKind::NewAth => "NEW ATH",
            AlertKind::FreshWallets => "NEW WALLETS",
        }
    }
}

#[derive(Clone)]
pub struct Alert {
    pub timestamp: String,
    pub kind: AlertKind,
    pub message: String,
}

pub struct AlertEngine {
    last_fired: HashMap<AlertKind, u64>,
    ath_fdv: f64,
    wallet_first_seen: HashMap<String, u64>,
    /// Rolling count of new wallets per 5s window for baseline
    fresh_wallet_baseline: f64,
    pub alerts: VecDeque<Alert>,
}

impl AlertEngine {
    pub fn new() -> Self {
        Self {
            last_fired: HashMap::new(),
            ath_fdv: 0.0,
            wallet_first_seen: HashMap::new(),
            fresh_wallet_baseline: 0.0,
            alerts: VecDeque::with_capacity(MAX_ALERTS),
        }
    }

    /// Evaluate signals. Takes individual fields to avoid borrow conflicts.
    pub fn evaluate(
        &mut self,
        now: u64,
        fdv_usd: Option<f64>,
        fdv_change_pct: Option<f64>,
        volume: &VolumeTracker,
        trade: Option<&TradeRow>,
    ) {
        let secs = now % 86400;
        let ts = format!("{:02}:{:02}:{:02}", secs / 3600, (secs % 3600) / 60, secs % 60);

        // Per-trade signals
        if let Some(t) = trade {
            self.wallet_first_seen.entry(t.wallet_short.clone()).or_insert(now);

            if t.is_buy && t.sol_amount >= 5.0 {
                self.fire(now, &ts, AlertKind::WhaleBuy,
                    format!("{:.2} SOL from {}", t.sol_amount, t.wallet_short));
            }
            if !t.is_buy && t.sol_amount >= 5.0 {
                self.fire(now, &ts, AlertKind::WhaleSell,
                    format!("{:.2} SOL from {}", t.sol_amount, t.wallet_short));
            }
        }

        // Volume surge / dying
        let accel = volume.volume_acceleration(now, 30);
        if accel > 2.5 {
            self.fire(now, &ts, AlertKind::VolumeSurge, format!("{:.1}x in 30s", accel));
        }
        if accel < 0.3 {
            let (b, s) = volume.count_in_window(now, 60);
            if b + s >= 5 {
                self.fire(now, &ts, AlertKind::VolumeDying, format!("{:.1}x — fading", accel));
            }
        }

        // Buy / sell wall
        let pressure = volume.buy_pressure(now, 30);
        let (b30, s30) = volume.count_in_window(now, 30);
        let total_30 = b30 + s30;
        if pressure > 0.80 && total_30 >= 8 {
            self.fire(now, &ts, AlertKind::BuyWall,
                format!("{:.0}% buys over {} trades", pressure * 100.0, total_30));
        }
        if pressure < 0.20 && total_30 >= 8 {
            self.fire(now, &ts, AlertKind::SellWall,
                format!("{:.0}% sells over {} trades", (1.0 - pressure) * 100.0, total_30));
        }

        // MCAP breakout / dump
        if let Some(pct) = fdv_change_pct {
            if pct > 15.0 {
                self.fire(now, &ts, AlertKind::McapBreakout, format!("+{:.1}% in 5s", pct));
            }
            if pct < -15.0 {
                self.fire(now, &ts, AlertKind::McapDump, format!("{:.1}% in 5s", pct));
            }
        }

        // New ATH
        if let Some(fdv) = fdv_usd {
            if self.ath_fdv == 0.0 {
                self.ath_fdv = fdv;
            } else if fdv > self.ath_fdv {
                self.ath_fdv = fdv;
                let label = if fdv >= 1_000_000.0 { format!("${:.2}M", fdv / 1_000_000.0) }
                    else if fdv >= 1_000.0 { format!("${:.1}k", fdv / 1_000.0) }
                    else { format!("${:.0}", fdv) };
                self.fire(now, &ts, AlertKind::NewAth, label);
            }
        }

        // Fresh wallets — spike detection vs rolling baseline
        // Count new wallets in last 5s
        let fresh_5s = self.wallet_first_seen.values()
            .filter(|&&seen| now.saturating_sub(seen) <= 5)
            .count() as f64;
        // Update baseline with exponential moving average (smoothing factor 0.15)
        self.fresh_wallet_baseline = self.fresh_wallet_baseline * 0.85 + fresh_5s * 0.15;
        // Spike: current rate > 2x baseline AND at least 3 new wallets
        let threshold = (self.fresh_wallet_baseline * 2.0).max(2.5);
        if fresh_5s >= threshold && fresh_5s >= 3.0 {
            self.fire(now, &ts, AlertKind::FreshWallets,
                format!("{} new wallets in 5s ({:.0}x avg)", fresh_5s as u32, fresh_5s / self.fresh_wallet_baseline.max(0.1)));
        }
    }

    fn fire(&mut self, now: u64, ts: &str, kind: AlertKind, message: String) {
        let last = self.last_fired.get(&kind).copied().unwrap_or(0);
        if now < last + kind.cooldown_secs() {
            return;
        }
        self.last_fired.insert(kind, now);
        self.alerts.push_front(Alert {
            timestamp: ts.to_string(),
            kind,
            message,
        });
        if self.alerts.len() > MAX_ALERTS {
            self.alerts.pop_back();
        }
    }
}
const MAX_FDV_POINTS: usize = 3600; // Keep up to 1 hour of raw data

/// Chart timeframe for the market cap candlestick chart
#[derive(Clone, Copy, PartialEq)]
pub enum ChartTimeframe {
    FiveSeconds,
    ThirtySeconds,
    OneMinute,
    FiveMinutes,
    FifteenMinutes,
}

impl ChartTimeframe {
    pub fn label(&self) -> &'static str {
        match self {
            ChartTimeframe::FiveSeconds => "5s",
            ChartTimeframe::ThirtySeconds => "30s",
            ChartTimeframe::OneMinute => "1m",
            ChartTimeframe::FiveMinutes => "5m",
            ChartTimeframe::FifteenMinutes => "15m",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            ChartTimeframe::FiveSeconds => ChartTimeframe::ThirtySeconds,
            ChartTimeframe::ThirtySeconds => ChartTimeframe::OneMinute,
            ChartTimeframe::OneMinute => ChartTimeframe::FiveMinutes,
            ChartTimeframe::FiveMinutes => ChartTimeframe::FifteenMinutes,
            ChartTimeframe::FifteenMinutes => ChartTimeframe::FiveSeconds,
        }
    }

    /// Bucket size in seconds
    pub fn bucket_secs(&self) -> u64 {
        match self {
            ChartTimeframe::FiveSeconds => 5,
            ChartTimeframe::ThirtySeconds => 30,
            ChartTimeframe::OneMinute => 60,
            ChartTimeframe::FiveMinutes => 300,
            ChartTimeframe::FifteenMinutes => 900,
        }
    }

    /// How many buckets to display
    pub fn num_buckets(&self) -> usize {
        match self {
            ChartTimeframe::FiveSeconds => 60,      // 5 minutes of 5s candles
            ChartTimeframe::ThirtySeconds => 60,     // 30 minutes of 30s candles
            ChartTimeframe::OneMinute => 60,         // 1 hour of 1m candles
            ChartTimeframe::FiveMinutes => 60,       // 5 hours of 5m candles
            ChartTimeframe::FifteenMinutes => 60,    // 15 hours of 15m candles
        }
    }
}

/// OHLC candlestick data for one time bucket
#[derive(Clone, Debug)]
pub struct Candle {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    /// True if this bucket had real trade data (not forward-filled)
    pub has_data: bool,
}

/// A single trade row for display
#[derive(Clone)]
pub struct TradeRow {
    pub timestamp: String,
    pub is_buy: bool,
    pub sol_amount: f64,
    pub wallet_short: String,
    pub epoch_secs: u64,
    pub mcap_usd: Option<f64>,
}

/// Rolling volume tracker with time-based windows
pub struct VolumeTracker {
    /// (epoch_secs, sol_amount, is_buy)
    entries: VecDeque<(u64, f64, bool)>,
}

impl VolumeTracker {
    pub fn new() -> Self {
        Self {
            entries: VecDeque::with_capacity(1000),
        }
    }

    pub fn push(&mut self, epoch_secs: u64, sol_amount: f64, is_buy: bool) {
        self.entries.push_back((epoch_secs, sol_amount, is_buy));
        // Prune entries older than 15 minutes
        let cutoff = epoch_secs.saturating_sub(900);
        while self.entries.front().map_or(false, |e| e.0 < cutoff) {
            self.entries.pop_front();
        }
    }

    /// Get (buy_volume, sell_volume) for the last `window_secs`
    pub fn volume_in_window(&self, now: u64, window_secs: u64) -> (f64, f64) {
        let cutoff = now.saturating_sub(window_secs);
        let mut buy = 0.0;
        let mut sell = 0.0;
        for &(ts, amount, is_buy) in &self.entries {
            if ts >= cutoff {
                if is_buy {
                    buy += amount;
                } else {
                    sell += amount;
                }
            }
        }
        (buy, sell)
    }

    /// Total trades in window
    pub fn count_in_window(&self, now: u64, window_secs: u64) -> (u32, u32) {
        let cutoff = now.saturating_sub(window_secs);
        let mut buys = 0u32;
        let mut sells = 0u32;
        for &(ts, _, is_buy) in &self.entries {
            if ts >= cutoff {
                if is_buy { buys += 1; } else { sells += 1; }
            }
        }
        (buys, sells)
    }

    /// Buy pressure ratio (0.0 = all sells, 1.0 = all buys) for a window
    pub fn buy_pressure(&self, now: u64, window_secs: u64) -> f64 {
        let (buy, sell) = self.volume_in_window(now, window_secs);
        let total = buy + sell;
        if total == 0.0 { 0.5 } else { buy / total }
    }

    /// Volume acceleration: ratio of recent volume vs older volume.
    /// >1.0 means accelerating, <1.0 means decelerating.
    /// Compares the most recent `window_secs` to the `window_secs` before that.
    pub fn volume_acceleration(&self, now: u64, window_secs: u64) -> f64 {
        let (buy_recent, sell_recent) = self.volume_in_window(now, window_secs);
        let recent = buy_recent + sell_recent;

        // Older window: from 2*window ago to 1*window ago
        let older_end = now.saturating_sub(window_secs);
        let older_start = older_end.saturating_sub(window_secs);
        let mut older = 0.0;
        for &(ts, amount, _) in &self.entries {
            if ts >= older_start && ts < older_end {
                older += amount;
            }
        }

        if older < 0.001 { return if recent > 0.001 { 2.0 } else { 1.0 }; }
        (recent / older).min(5.0) // cap at 5x
    }

    /// Transactions per second over the last `window_secs`
    pub fn tps(&self, now: u64, window_secs: u64) -> f64 {
        let (buys, sells) = self.count_in_window(now, window_secs);
        let total = buys + sells;
        if window_secs == 0 { return 0.0; }
        total as f64 / window_secs as f64
    }

    /// Volume percentage change: compare window [now-secs..now] vs [now-2*secs..now-secs]
    pub fn volume_change_pct(&self, now: u64, window_secs: u64) -> f64 {
        let (buy_recent, sell_recent) = self.volume_in_window(now, window_secs);
        let recent = buy_recent + sell_recent;

        let older_end = now.saturating_sub(window_secs);
        let older_start = older_end.saturating_sub(window_secs);
        let mut older = 0.0;
        for &(ts, amount, _) in &self.entries {
            if ts >= older_start && ts < older_end {
                older += amount;
            }
        }

        if older < 0.001 {
            if recent > 0.001 { 100.0 } else { 0.0 }
        } else {
            ((recent - older) / older * 100.0).clamp(-999.0, 999.0)
        }
    }
}

/// Application state for the TUI
pub struct App {
    pub token_symbol: String,
    pub token_name: String,
    pub mint_address: String,
    pub sol_price: f64,
    pub fdv_usd: Option<f64>,
    pub trades: VecDeque<TradeRow>,
    pub tx_count: u64,
    pub connected: bool,
    pub start_time: Instant,
    pub last_update: Option<Instant>,
    pub unique_wallets: std::collections::HashSet<String>,
    pub largest_trade_sol: f64,

    // Cumulative transaction fees (SOL) since session start
    pub total_fees_sol: f64,

    // Price sparkline data (legacy, kept for compatibility)
    pub price_history: VecDeque<f64>,

    // FDV (Market Cap) history: (epoch_secs, fdv_usd)
    pub fdv_history: VecDeque<(u64, f64)>,

    // Chart timeframe selection
    pub chart_timeframe: ChartTimeframe,

    // Volume tracking
    pub volume: VolumeTracker,

    // Current epoch_secs for time calculations
    pub now_epoch: u64,

    // FDV 5-second change tracking
    pub fdv_5s_ago: Option<f64>,
    pub fdv_5s_snapshot_time: u64,

    // Alert engine
    pub alert_engine: AlertEngine,

    // View mode: Normal (Bloomberg 4-panel) or Alert (chart-dominant + alerts)
    pub view_mode: ViewMode,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ViewMode {
    Normal,
    Alert,
}

impl App {
    pub fn new(symbol: String, name: String, mint: String) -> Self {
        Self {
            token_symbol: symbol,
            token_name: name,
            mint_address: mint,
            sol_price: 140.0,
            fdv_usd: None,
            trades: VecDeque::with_capacity(MAX_TRADES),
            tx_count: 0,
            connected: false,
            start_time: Instant::now(),
            last_update: None,
            unique_wallets: std::collections::HashSet::new(),
            largest_trade_sol: 0.0,
            total_fees_sol: 0.0,
            price_history: VecDeque::with_capacity(MAX_PRICE_POINTS),
            fdv_history: VecDeque::with_capacity(MAX_FDV_POINTS),
            chart_timeframe: ChartTimeframe::OneMinute,
            volume: VolumeTracker::new(),
            now_epoch: 0,
            fdv_5s_ago: None,
            fdv_5s_snapshot_time: 0,
            alert_engine: AlertEngine::new(),
            view_mode: ViewMode::Normal,
        }
    }

    pub fn push_trade(&mut self, row: TradeRow) {
        self.now_epoch = row.epoch_secs;
        self.last_update = Some(Instant::now());
        self.unique_wallets.insert(row.wallet_short.clone());
        if row.sol_amount > self.largest_trade_sol {
            self.largest_trade_sol = row.sol_amount;
        }
        self.volume.push(row.epoch_secs, row.sol_amount, row.is_buy);
        self.trades.push_front(row);
        if self.trades.len() > MAX_TRADES {
            self.trades.pop_back();
        }
    }

    pub fn push_price(&mut self, price: f64) {
        self.price_history.push_back(price);
        if self.price_history.len() > MAX_PRICE_POINTS {
            self.price_history.pop_front();
        }
    }

    /// Push an FDV data point with timestamp
    pub fn push_fdv_point(&mut self, epoch_secs: u64, fdv_usd: f64) {
        self.fdv_history.push_back((epoch_secs, fdv_usd));
        if self.fdv_history.len() > MAX_FDV_POINTS {
            self.fdv_history.pop_front();
        }
    }

    /// FDV percentage change over the last ~5 seconds
    pub fn fdv_change_pct(&self) -> Option<f64> {
        match (self.fdv_usd, self.fdv_5s_ago) {
            (Some(current), Some(old)) if old > 0.0 => Some((current - old) / old * 100.0),
            _ => None,
        }
    }

    /// Apply a TelemetryEvent from the EventBus.
    /// This is the sole way the TUI receives data — it does not own engines or streams.
    pub fn apply_event(&mut self, event: &crate::events::TelemetryEvent) {
        use crate::events::TelemetryEvent;
        match event {
            TelemetryEvent::Trade {
                timestamp, epoch_secs, is_buy, sol_amount, price,
                wallet_short, fdv_usd, ..
            } => {
                let row = TradeRow {
                    timestamp: timestamp.clone(),
                    is_buy: *is_buy,
                    sol_amount: *sol_amount,
                    wallet_short: wallet_short.clone(),
                    epoch_secs: *epoch_secs,
                    mcap_usd: *fdv_usd,
                };
                self.push_trade(row.clone());
                if *price > 0.0 {
                    self.push_price(*price);
                }
                // Evaluate alert signals
                let fdv_pct = self.fdv_change_pct();
                self.alert_engine.evaluate(
                    self.now_epoch, self.fdv_usd, fdv_pct, &self.volume, Some(&row),
                );
            }
            TelemetryEvent::FdvUpdate { epoch_secs, fdv_usd } => {
                // Snapshot current FDV every ~5 seconds for % change tracking
                if *epoch_secs >= self.fdv_5s_snapshot_time + 5 {
                    self.fdv_5s_ago = self.fdv_usd;
                    self.fdv_5s_snapshot_time = *epoch_secs;
                }
                self.fdv_usd = Some(*fdv_usd);
                self.push_fdv_point(*epoch_secs, *fdv_usd);
            }
            TelemetryEvent::SolPriceUpdate { price } => {
                self.sol_price = *price;
            }
            TelemetryEvent::FeeAccrued { fee_sol } => {
                self.total_fees_sol += fee_sol;
            }
            TelemetryEvent::TxProcessed => {
                self.tx_count += 1;
            }
        }
    }

    /// Get OHLC data. Returns (candles, global_low, global_high, total_slots).
    /// `total_slots` is always num_buckets — the renderer uses it to position
    /// candles at fixed time slots so the chart grows left-to-right like a static camera.
    pub fn get_ohlc_data(&self) -> (Vec<(usize, Candle)>, f64, f64, usize) {
        if self.fdv_history.is_empty() {
            return (vec![], 0.0, 0.0, 0);
        }

        let bucket_secs = self.chart_timeframe.bucket_secs();
        let num_buckets = self.chart_timeframe.num_buckets();
        let now = self.now_epoch;

        struct BucketBuilder {
            first_ts: u64,
            open: f64,
            high: f64,
            low: f64,
            close: f64,
            last_ts: u64,
        }

        let mut builders: Vec<Option<BucketBuilder>> = (0..num_buckets).map(|_| None).collect();

        for &(ts, fdv) in &self.fdv_history {
            if ts > now { continue; }
            let age_secs = now.saturating_sub(ts);
            let bucket_age = age_secs / bucket_secs;
            if (bucket_age as usize) >= num_buckets { continue; }
            let idx = num_buckets - 1 - bucket_age as usize;

            match &mut builders[idx] {
                Some(b) => {
                    if ts < b.first_ts { b.first_ts = ts; b.open = fdv; }
                    if ts > b.last_ts { b.last_ts = ts; b.close = fdv; }
                    if fdv > b.high { b.high = fdv; }
                    if fdv < b.low { b.low = fdv; }
                }
                None => {
                    builders[idx] = Some(BucketBuilder {
                        first_ts: ts, open: fdv, high: fdv,
                        low: fdv, close: fdv, last_ts: ts,
                    });
                }
            }
        }

        // Build indexed candles: (slot_index, Candle)
        // Each candle keeps its slot position so the renderer can place it correctly.
        // Gap-fill candles are included so the chart line is continuous.
        let mut candles: Vec<(usize, Candle)> = Vec::with_capacity(num_buckets);
        let mut last_close: Option<f64> = None;
        let mut first_data_seen = false;

        for (idx, b) in builders.into_iter().enumerate() {
            match b {
                Some(bb) => {
                    first_data_seen = true;
                    last_close = Some(bb.close);
                    candles.push((idx, Candle {
                        open: bb.open, high: bb.high,
                        low: bb.low, close: bb.close,
                        has_data: true,
                    }));
                }
                None => {
                    if first_data_seen {
                        if let Some(lc) = last_close {
                            candles.push((idx, Candle {
                                open: lc, high: lc, low: lc, close: lc,
                                has_data: false,
                            }));
                        }
                    }
                    // Leading gaps before any data — skip entirely
                }
            }
        }

        if candles.is_empty() {
            return (vec![], 0.0, 0.0, num_buckets);
        }

        // Y range from real candles only
        let global_low = candles.iter()
            .filter(|(_, c)| c.has_data)
            .map(|(_, c)| c.low)
            .fold(f64::INFINITY, f64::min);
        let global_high = candles.iter()
            .filter(|(_, c)| c.has_data)
            .map(|(_, c)| c.high)
            .fold(f64::NEG_INFINITY, f64::max);

        let global_low = if global_low.is_infinite() { candles[0].1.close } else { global_low };
        let global_high = if global_high.is_infinite() { candles[0].1.close } else { global_high };

        (candles, global_low, global_high, num_buckets)
    }

    /// Uptime as formatted string
    pub fn uptime(&self) -> String {
        let secs = self.start_time.elapsed().as_secs();
        if secs < 60 {
            format!("{}s", secs)
        } else if secs < 3600 {
            format!("{}m {}s", secs / 60, secs % 60)
        } else {
            format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
        }
    }

    /// Seconds since last update
    pub fn secs_since_update(&self) -> Option<u64> {
        self.last_update.map(|t| t.elapsed().as_secs())
    }
}

/// Fetch SOL/USD price, trying multiple sources
pub async fn fetch_sol_price() -> Option<f64> {
    // Build a client that works regardless of TLS provider
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .ok()?;

    // Try CoinGecko first
    if let Some(price) = fetch_from_coingecko(&client).await {
        return Some(price);
    }

    // Fallback: Binance public API
    if let Some(price) = fetch_from_binance(&client).await {
        return Some(price);
    }

    None
}

async fn fetch_from_coingecko(client: &reqwest::Client) -> Option<f64> {
    let url = "https://api.coingecko.com/api/v3/simple/price?ids=solana&vs_currencies=usd";
    let resp = client.get(url).send().await.ok()?;
    let json: serde_json::Value = resp.json().await.ok()?;
    json["solana"]["usd"].as_f64()
}

async fn fetch_from_binance(client: &reqwest::Client) -> Option<f64> {
    let url = "https://api.binance.com/api/v3/ticker/price?symbol=SOLUSDT";
    let resp = client.get(url).send().await.ok()?;
    let json: serde_json::Value = resp.json().await.ok()?;
    json["price"].as_str()?.parse::<f64>().ok()
}
