//! The runtime play plan: how many games the batch has played, how many are
//! left, and when the batch must stop.
//!
//! The plan is deliberately **not** a batch that restarts whenever its limit
//! changes. The user lowers the limit to stop earlier and raises it to go on,
//! and a counter that reset on every edit would make "已打 N / 还剩 M"
//! meaningless — worse, it would turn "stop after the game in play" into
//! "play N more games". The counter therefore only restarts when the user
//! asks for it explicitly, and it survives a restart of the app through a
//! small file next to the config.
//!
//! Two stop conditions, both checked *between* games (never mid-game — a
//! game in play is always finished):
//!
//! - a **budget** — stop after N finished games (`max_games`, `0` = no
//!   budget);
//! - a **cutoff** — a daily wall-clock moment after which no new game is
//!   started (`stop_at`, `HH:MM` local time).
//!
//! Both live in the config, so both can be changed while the bot runs; the
//! overlay and the Settings page write them through the same commands.

use chrono::{Local, Timelike};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

/// The live counter, shared between the autoplay manager (which advances it
/// after every finished game) and the IPC commands (which read it and let
/// the user change it).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanState {
    /// Games finished since the counter was last reset.
    pub games: u32,
}

/// Shared handle to the counter.
pub type SharedPlan = Arc<Mutex<PlanState>>;

/// The plan as the frontend sees it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequeueStatus {
    /// Games finished since the counter was last reset.
    pub games: u32,
    /// The game budget. `0` = unlimited.
    pub max_games: u32,
    /// Games still to play before the budget stops the loop. `None` when the
    /// budget is unlimited.
    pub remaining: Option<u32>,
    /// Daily wall-clock cutoff, `HH:MM` local time. `None` = no cutoff.
    pub stop_at: Option<String>,
    /// Whether the cutoff has already passed today.
    pub deadline_passed: bool,
}

impl PlanState {
    /// The status the frontend renders: the counter plus the two stop
    /// conditions, which live in the config rather than in the counter.
    pub fn status(&self, max_games: u32, stop_at: Option<String>) -> RequeueStatus {
        let remaining = if max_games == 0 {
            None
        } else {
            Some(max_games.saturating_sub(self.games))
        };
        let deadline_passed = stop_at
            .as_deref()
            .and_then(parse_deadline)
            .map(|d| deadline_passed_at(d, Local::now()))
            .unwrap_or(false);
        RequeueStatus {
            games: self.games,
            max_games,
            remaining,
            stop_at,
            deadline_passed,
        }
    }
}

/// Parse a `HH:MM` local wall-clock cutoff into minutes since midnight.
/// Seconds are accepted and ignored, so a value from a seconds-bearing time
/// picker still works. `None` for anything unparseable — the caller warns
/// and carries on rather than refusing to play.
pub fn parse_deadline(raw: &str) -> Option<u32> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    let mut parts = s.split(':');
    let hours: u32 = parts.next()?.trim().parse().ok()?;
    let minutes: u32 = parts.next()?.trim().parse().ok()?;
    if let Some(seconds) = parts.next() {
        seconds.trim().parse::<u32>().ok()?;
    }
    if parts.next().is_some() || hours > 23 || minutes > 59 {
        return None;
    }
    Some(hours * 60 + minutes)
}

/// Whether the cutoff has passed.
///
/// The cutoff is a *daily* moment: no new game is started at or after it,
/// and the next day it applies again. A cutoff of `03:00` therefore means
/// "play until three in the morning" when it is set at one in the morning,
/// and "do not start anything after three in the morning" the rest of the
/// day.
pub fn deadline_passed_at(deadline_minutes: u32, now: chrono::DateTime<Local>) -> bool {
    let now_minutes = now.hour() * 60 + now.minute();
    now_minutes >= deadline_minutes
}

/// The budget that makes the game in play the last one.
///
/// The counter holds the games that have *finished*, so the game running
/// now is number `games + 1` — and a budget of exactly that many makes it
/// the last one.
pub fn budget_for_current_game(games: u32) -> u32 {
    games.saturating_add(1)
}

/// The counter file, next to the config so the plan survives a restart.
pub fn progress_path(config_dir: &Path) -> PathBuf {
    config_dir.join("requeue_progress.json")
}

/// What is written to [`progress_path`].
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedPlan {
    games: u32,
}

/// Read the counter back. A missing or unreadable file means zero — the
/// plan simply starts over rather than refusing to run.
pub async fn load_games(path: &Path) -> u32 {
    match tokio::fs::read_to_string(path).await {
        Ok(body) => serde_json::from_str::<PersistedPlan>(&body)
            .map(|p| p.games)
            .unwrap_or(0),
        Err(_) => 0,
    }
}

/// Write the counter back. A failure is logged, not fatal: the plan is a
/// convenience, and a read-only disk must not stop the bot from playing.
pub async fn save_games(path: &Path, games: u32) {
    let body = serde_json::to_string_pretty(&PersistedPlan { games }).unwrap_or_default();
    if let Err(e) = tokio::fs::write(path, body).await {
        tracing::warn!(
            "requeue: could not save the progress file at {}: {e}",
            path.display()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(hour: u32, minute: u32) -> chrono::DateTime<Local> {
        Local.with_ymd_and_hms(2026, 11, 21, hour, minute, 0).unwrap()
    }

    #[test]
    fn a_plain_hour_and_minute_parses() {
        assert_eq!(parse_deadline("23:30"), Some(23 * 60 + 30));
        assert_eq!(parse_deadline("00:00"), Some(0));
        assert_eq!(parse_deadline("03:05"), Some(3 * 60 + 5));
    }

    #[test]
    fn surrounding_space_and_seconds_are_tolerated() {
        assert_eq!(parse_deadline("  23:30 "), Some(23 * 60 + 30));
        assert_eq!(parse_deadline("23:30:15"), Some(23 * 60 + 30));
    }

    #[test]
    fn nonsense_is_rejected_rather_than_guessed() {
        assert_eq!(parse_deadline(""), None);
        assert_eq!(parse_deadline("   "), None);
        assert_eq!(parse_deadline("23"), None);
        assert_eq!(parse_deadline("24:00"), None);
        assert_eq!(parse_deadline("12:60"), None);
        assert_eq!(parse_deadline("half past ten"), None);
        assert_eq!(parse_deadline("23:30:00:00"), None);
    }

    #[test]
    fn the_cutoff_applies_from_the_moment_itself() {
        let deadline = parse_deadline("03:00").unwrap();
        assert!(!deadline_passed_at(deadline, at(2, 59)));
        assert!(deadline_passed_at(deadline, at(3, 0)));
        assert!(deadline_passed_at(deadline, at(3, 1)));
    }

    #[test]
    fn a_late_night_cutoff_does_not_fire_just_after_midnight() {
        // "23:30" means "do not start anything after half past eleven".
        // One in the morning is before that, so the plan still runs.
        let deadline = parse_deadline("23:30").unwrap();
        assert!(!deadline_passed_at(deadline, at(0, 10)));
        assert!(deadline_passed_at(deadline, at(23, 45)));
    }

    #[test]
    fn the_remaining_count_follows_the_counter() {
        let state = PlanState { games: 12 };
        let status = state.status(20, None);
        assert_eq!(status.games, 12);
        assert_eq!(status.remaining, Some(8));
        assert!(!status.deadline_passed);
    }

    #[test]
    fn an_unlimited_budget_has_no_remaining_count() {
        let status = PlanState { games: 99 }.status(0, None);
        assert_eq!(status.remaining, None);
        assert_eq!(status.max_games, 0);
    }

    #[test]
    fn a_counter_past_its_budget_does_not_underflow() {
        let status = PlanState { games: 25 }.status(20, None);
        assert_eq!(status.remaining, Some(0));
    }

    #[test]
    fn the_budget_for_the_game_in_play_is_the_counter_plus_one() {
        // The counter holds finished games, so the game running now is the
        // next one — a budget of exactly that makes it the last one.
        assert_eq!(budget_for_current_game(0), 1);
        assert_eq!(budget_for_current_game(12), 13);
        assert_eq!(budget_for_current_game(u32::MAX), u32::MAX);
    }

    #[tokio::test]
    async fn the_progress_file_round_trips() {
        let tmp = std::env::temp_dir().join(format!(
            "akagi-plan-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        tokio::fs::create_dir_all(&tmp).await.unwrap();
        let path = progress_path(&tmp);
        assert_eq!(load_games(&path).await, 0);
        save_games(&path, 7).await;
        assert_eq!(load_games(&path).await, 7);
        let _ = tokio::fs::remove_dir_all(&tmp).await;
    }
}
