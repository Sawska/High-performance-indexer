//! Live progress of the scrape loop.
//!
//! The scraper reports into one process-wide snapshot instead of threading a
//! handle through every stage: it is observability only, nothing reads it back
//! to make decisions, so a global keeps the stage signatures untouched.
//!
//! Served to the UI by `GET /api/status` (src/api/mod.rs).

use std::sync::{LazyLock, Mutex};

use chrono::{DateTime, Utc};
use serde::Serialize;

/// How many recent error lines to keep for the UI.
const ERROR_TAIL: usize = 20;

#[derive(Clone, Serialize)]
pub struct Stage {
    pub name: &'static str,
    pub done: u64,
    /// 0 when the stage cannot know its size up front (the market walk).
    pub total: u64,
    pub errors: u64,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Serialize)]
pub struct ErrorLine {
    pub at: DateTime<Utc>,
    pub msg: String,
}

#[derive(Clone, Serialize)]
pub struct Snapshot {
    pub process_started_at: DateTime<Utc>,
    pub cycle: u64,
    pub cycles_completed: u64,
    pub cycle_started_at: Option<DateTime<Utc>>,
    pub last_cycle_ms: Option<u64>,
    pub last_cycle_finished_at: Option<DateTime<Utc>>,
    /// Counts of what the current cycle found.
    pub markets: u64,
    pub live_markets: u64,
    pub tokens: u64,
    pub wallets_discovered: u64,
    pub wallets_this_cycle: u64,
    /// Stages of the current cycle, in the order they started.
    pub stages: Vec<Stage>,
    pub errors_total: u64,
    pub recent_errors: Vec<ErrorLine>,
}

static SNAPSHOT: LazyLock<Mutex<Snapshot>> = LazyLock::new(|| {
    Mutex::new(Snapshot {
        process_started_at: Utc::now(),
        cycle: 0,
        cycles_completed: 0,
        cycle_started_at: None,
        last_cycle_ms: None,
        last_cycle_finished_at: None,
        markets: 0,
        live_markets: 0,
        tokens: 0,
        wallets_discovered: 0,
        wallets_this_cycle: 0,
        stages: Vec::new(),
        errors_total: 0,
        recent_errors: Vec::new(),
    })
});

fn with<R>(f: impl FnOnce(&mut Snapshot) -> R) -> R {
    // A panic elsewhere while holding the lock must not take the stats down too.
    let mut s = SNAPSHOT.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut s)
}

fn close_open_stage(s: &mut Snapshot, now: DateTime<Utc>) {
    if let Some(last) = s.stages.last_mut() {
        last.finished_at.get_or_insert(now);
    }
}

pub fn cycle_start(cycle: u64) {
    with(|s| {
        s.cycle = cycle;
        s.cycle_started_at = Some(Utc::now());
        s.markets = 0;
        s.live_markets = 0;
        s.tokens = 0;
        s.wallets_discovered = 0;
        s.wallets_this_cycle = 0;
        s.stages.clear();
    });
}

pub fn cycle_end(elapsed_ms: u64) {
    with(|s| {
        let now = Utc::now();
        close_open_stage(s, now);
        s.cycles_completed += 1;
        s.last_cycle_ms = Some(elapsed_ms);
        s.last_cycle_finished_at = Some(now);
    });
}

pub fn stage_start(name: &'static str, total: usize) {
    with(|s| {
        let now = Utc::now();
        close_open_stage(s, now);
        s.stages.push(Stage {
            name,
            done: 0,
            total: total as u64,
            errors: 0,
            started_at: now,
            finished_at: None,
        });
    });
}

/// One unit of the current stage finished, successfully or not.
pub fn tick() {
    with(|s| {
        if let Some(stage) = s.stages.last_mut() {
            stage.done += 1;
        }
    });
}

pub fn set(f: impl FnOnce(&mut Snapshot)) {
    with(f);
}

pub fn error(msg: String) {
    with(|s| {
        s.errors_total += 1;
        if let Some(stage) = s.stages.last_mut() {
            stage.errors += 1;
        }
        if s.recent_errors.len() == ERROR_TAIL {
            s.recent_errors.remove(0);
        }
        s.recent_errors.push(ErrorLine {
            at: Utc::now(),
            msg,
        });
    });
}

pub fn snapshot() -> Snapshot {
    with(|s| s.clone())
}
