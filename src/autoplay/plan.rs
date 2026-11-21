//! The runtime play plan: how many games the batch has played, how many are
//! left, and when the batch must stop.
//!
//! The plan is deliberately **not** a batch that restarts whenever its limit
//! changes. The user lowers the limit to stop earlier and raises it to go
//! on, and a counter that reset on every edit would make "已打 N / 还剩 M"
//! meaningless — worse, it would turn "stop after the game in play" into
//! "play N more games". The counter therefore only restarts when the user
//! asks for it explicitly, and it survives a restart of the app through a
//! small file next to the config.
//!
//! Two stop conditions, both checked **between** games (never mid-game — a
//! game in play is always finished):
//!
//! - a **budget** — stop after N finished games (`max_games`, `0` = no
//!   budget);
//! - a **deadline** — an absolute wall-clock moment after which no new
//!   game is started (`stop_at`).
//!
//! The deadline is absolute, not a daily rule, and that is deliberate: a
//! deadline that re-derived itself from "now" on every read would keep
//! moving and the bot would never stop. It is stored as
//! `YYYY-MM-DDTHH:MM` in the config, and the UI shows it as a clock plus
//! the number of days left ("还差 2 天"), which is how the user thinks
//! about it.

use chrono::{DateTime, Duration, Local, NaiveDate, NaiveDateTime};
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
    /// The deadline as it is stored: an absolute `YYYY-MM-DDTHH:MM` local
    /// moment. `None` = no deadline.
    pub stop_at: Option<String>,
    /// The deadline's clock, `HH:MM` — what the time field shows.
    pub stop_clock: Option<String>,
    /// How many days from today to the deadline: `0` = today, `1` = tomorrow.
    /// Negative when the deadline is behind us.
    pub stop_days: Option<i32>,
    /// Whether the deadline has passed.
    pub deadline_passed: bool,
}

impl PlanState {
    /// The status the frontend renders: the counter plus the two stop
    /// conditions, which live in the config rather than in the counter.
    pub fn status(
        &self,
        max_games: u32,
        stop_at: Option<String>,
        now: DateTime<Local>,
    ) -> RequeueStatus {
        let remaining = if max_games == 0 {
            None
        } else {
            Some(max_games.saturating_sub(self.games))
        };
        let parsed = stop_at.as_deref().and_then(parse_deadline);
        let (stop_clock, stop_days, deadline_passed) = match parsed {
            Some(d) => (
                Some(clock_of(d)),
                Some(days_until(d, now.date_naive())),
                deadline_passed(d, now),
            ),
            None => (None, None, false),
        };
        RequeueStatus {
            games: self.games,
            max_games,
            remaining,
            stop_at,
            stop_clock,
            stop_days,
            deadline_passed,
        }
    }
}

/// Parse an absolute deadline, `YYYY-MM-DDTHH:MM` (a space instead of the
/// `T` is accepted too). `None` for anything unparseable — the caller warns
/// and carries on rather than refusing to play.
pub fn parse_deadline(raw: &str) -> Option<NaiveDateTime> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    let s = s.replacen(' ', "T", 1);
    for fmt in ["%Y-%m-%dT%H:%M", "%Y-%m-%dT%H:%M:%S"] {
        if let Ok(dt) = NaiveDateTime::parse_from_str(&s, fmt) {
            return Some(dt);
        }
    }
    None
}

/// The absolute deadline as it is written to the config.
pub fn format_deadline(dt: NaiveDateTime) -> String {
    dt.format("%Y-%m-%dT%H:%M").to_string()
}

/// The clock part of a deadline, `HH:MM`.
pub fn clock_of(dt: NaiveDateTime) -> String {
    dt.format("%H:%M").to_string()
}

/// Parse a `HH:MM` clock. `None` for anything unparseable.
pub fn parse_clock(raw: &str) -> Option<(u32, u32)> {
    let s = raw.trim();
    let mut parts = s.split(':');
    let hours: u32 = parts.next()?.trim().parse().ok()?;
    let minutes: u32 = parts.next()?.trim().parse().ok()?;
    if parts.next().is_some() || hours > 23 || minutes > 59 {
        return None;
    }
    Some((hours, minutes))
}

/// How many days from `today` to the deadline — `0` = today, `1` =
/// tomorrow, negative when the deadline is behind us.
pub fn days_until(deadline: NaiveDateTime, today: NaiveDate) -> i32 {
    (deadline.date() - today).num_days() as i32
}

/// Whether the deadline has passed.
pub fn deadline_passed(deadline: NaiveDateTime, now: DateTime<Local>) -> bool {
    now.naive_local() >= deadline
}

/// Turn a clock plus a day offset into an absolute deadline.
///
/// The offset counts from **today**: `0` = today, `1` = tomorrow, `2` = the
/// day after. When today's clock has already gone by, "today at that clock"
/// is impossible, so the next occurrence is taken — which is exactly where
/// "it is 10:10 now and you set 10:09" turns into "another 24 hours".
///
/// One exception keeps a running plan from being pushed forward forever: if
/// the stored deadline is already today **and the clock was not changed**,
/// it stays where it is (expired) instead of rolling over to tomorrow. The
/// user can still move it on explicitly.
///
/// `stored` is the deadline currently in the config, if any.
pub fn resolve_deadline(
    clock: &str,
    days: u32,
    now: DateTime<Local>,
    stored: Option<NaiveDateTime>,
) -> Option<NaiveDateTime> {
    let (hours, minutes) = parse_clock(clock)?;
    let today = now.date_naive();
    // Midnight of the chosen day, then the clock on top of it. The offset is
    // counted in whole days first, so "+1 day" always means the next
    // calendar day rather than "24 hours from now".
    let midnight = today.and_hms_opt(0, 0, 0)?;
    let mut candidate = midnight
        + Duration::days(days as i64)
        + Duration::hours(hours as i64)
        + Duration::minutes(minutes as i64);
    if candidate <= now.naive_local() {
        if let Some(kept) = stored {
            if kept.date() == today && clock_of(kept) == clock.trim() {
                return Some(kept);
            }
        }
        candidate += Duration::days(1);
    }
    Some(candidate)
}

/// The counter file, next to the config so the plan survives a restart.
pub fn progress_path(config_dir: &Path) -> PathBuf {
    config_dir.join("requeue_progress.json")
}

/// The budget that makes the game in play the last one.
///
/// The counter holds the games that have *finished*, so the game running
/// now is number `games + 1` — and a budget of exactly that many makes it
/// the last one.
pub fn budget_for_current_game(games: u32) -> u32 {
    games.saturating_add(1)
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

    fn at(hour: u32, minute: u32) -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 11, 21, hour, minute, 0).unwrap()
    }

    fn day(d: u32, hour: u32, minute: u32) -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 11, d, hour, minute, 0).unwrap()
    }

    #[test]
    fn an_absolute_deadline_parses() {
        let d = parse_deadline("2026-11-24T23:30").unwrap();
        assert_eq!(d.format("%Y-%m-%d %H:%M").to_string(), "2026-11-24 23:30");
        assert_eq!(
            parse_deadline("2026-11-24 23:30").unwrap(),
            d,
            "a space is accepted for the T"
        );
        assert_eq!(parse_deadline("2026-11-24T23:30:15").unwrap().format("%H:%M").to_string(), "23:30");
    }

    #[test]
    fn nonsense_deadlines_are_rejected() {
        assert_eq!(parse_deadline(""), None);
        assert_eq!(parse_deadline("   "), None);
        assert_eq!(parse_deadline("23:30"), None, "a clock alone is not a deadline");
        assert_eq!(parse_deadline("2026-13-01T00:00"), None);
        assert_eq!(parse_deadline("tomorrow"), None);
    }

    #[test]
    fn the_day_offset_counts_from_today() {
        // Today (the 21st) plus two days is the 23rd.
        let d = resolve_deadline("23:30", 2, at(10, 10), None).unwrap();
        assert_eq!(d.format("%Y-%m-%d %H:%M").to_string(), "2026-11-23 23:30");
        assert_eq!(days_until(d, NaiveDate::from_ymd_opt(2026, 11, 21).unwrap()), 2);
        assert_eq!(days_until(d, NaiveDate::from_ymd_opt(2026, 11, 22).unwrap()), 1);
        assert_eq!(days_until(d, NaiveDate::from_ymd_opt(2026, 11, 23).unwrap()), 0);
    }

    #[test]
    fn a_clock_that_has_gone_by_means_the_next_day() {
        // 10:10 now, "10:09" → today's 10:09 is gone → tomorrow 10:09, i.e.
        // roughly another 24 hours.
        let d = resolve_deadline("10:09", 0, at(10, 10), None).unwrap();
        assert_eq!(d.format("%Y-%m-%d %H:%M").to_string(), "2026-11-22 10:09");
    }

    #[test]
    fn a_future_clock_stays_today() {
        let d = resolve_deadline("23:30", 0, at(10, 10), None).unwrap();
        assert_eq!(d.format("%Y-%m-%d %H:%M").to_string(), "2026-11-21 23:30");
    }

    #[test]
    fn an_expired_deadline_is_not_pushed_on_by_a_re_save() {
        // The stored deadline is today at 23:30 and it is now 23:40. Saving
        // the same clock again must leave it there — otherwise the plan
        // would roll over every time and never end.
        let stored = parse_deadline("2026-11-21T23:30").unwrap();
        let d = resolve_deadline("23:30", 0, at(23, 40), Some(stored)).unwrap();
        assert_eq!(d, stored);
    }

    #[test]
    fn changing_the_clock_moves_an_expired_deadline() {
        // Same moment, but the user asks for a different clock — that is a
        // deliberate change, so the usual "next occurrence" rule applies.
        let stored = parse_deadline("2026-11-21T23:30").unwrap();
        let d = resolve_deadline("10:00", 0, at(23, 40), Some(stored)).unwrap();
        assert_eq!(d.format("%Y-%m-%d %H:%M").to_string(), "2026-11-22 10:00");
    }

    #[test]
    fn three_days_from_the_second_is_the_fifth() {
        // The user's own example: on the 2nd, "+3 days" means the 5th.
        let d = resolve_deadline("23:30", 3, day(2, 10, 10), None).unwrap();
        assert_eq!(d.format("%Y-%m-%d %H:%M").to_string(), "2026-11-05 23:30");
    }

    #[test]
    fn re_saving_the_clock_keeps_the_stored_day() {
        // Set on the 2nd with "+3 days" → the 5th. On the 3rd the offset reads
        // 2, and saving the clock again must resolve back to the 5th — not to
        // the 6th. Otherwise every save would push the plan a day further.
        let stored = parse_deadline("2026-11-05T23:30").unwrap();
        let d = resolve_deadline("23:30", 2, day(3, 10, 10), Some(stored)).unwrap();
        assert_eq!(d, stored);
    }

    #[test]
    fn the_days_left_count_down_on_their_own() {
        // Set on the 2nd with "+3 days" → the deadline is the 5th. From then
        // on the offset counts down on its own: 2 days on the 3rd, 1 on the
        // 4th, and none on the 5th — the day it stops.
        let deadline = parse_deadline("2026-11-05T23:30").unwrap();
        let d2 = NaiveDate::from_ymd_opt(2026, 11, 2).unwrap();
        let d3 = NaiveDate::from_ymd_opt(2026, 11, 3).unwrap();
        let d4 = NaiveDate::from_ymd_opt(2026, 11, 4).unwrap();
        let d5 = NaiveDate::from_ymd_opt(2026, 11, 5).unwrap();
        assert_eq!(days_until(deadline, d2), 3);
        assert_eq!(days_until(deadline, d3), 2);
        assert_eq!(days_until(deadline, d4), 1);
        assert_eq!(days_until(deadline, d5), 0);
    }

    #[test]
    fn the_deadline_passes_at_its_moment() {
        let d = parse_deadline("2026-11-21T03:00").unwrap();
        assert!(!deadline_passed(d, at(2, 59)));
        assert!(deadline_passed(d, at(3, 0)));
        assert!(deadline_passed(d, at(3, 1)));
    }

    #[test]
    fn a_clock_parses() {
        assert_eq!(parse_clock("23:30"), Some((23, 30)));
        assert_eq!(parse_clock(" 03:05 "), Some((3, 5)));
        assert_eq!(parse_clock("24:00"), None);
        assert_eq!(parse_clock("12:60"), None);
        assert_eq!(parse_clock("12"), None);
    }

    #[test]
    fn the_remaining_count_follows_the_counter() {
        let state = PlanState { games: 12 };
        let status = state.status(20, None, at(10, 10));
        assert_eq!(status.games, 12);
        assert_eq!(status.remaining, Some(8));
        assert!(!status.deadline_passed);
        assert_eq!(status.stop_clock, None);
    }

    #[test]
    fn an_unlimited_budget_has_no_remaining_count() {
        let status = PlanState { games: 99 }.status(0, None, at(10, 10));
        assert_eq!(status.remaining, None);
        assert_eq!(status.max_games, 0);
    }

    #[test]
    fn a_counter_past_its_budget_does_not_underflow() {
        let status = PlanState { games: 25 }.status(20, None, at(10, 10));
        assert_eq!(status.remaining, Some(0));
    }

    #[test]
    fn the_status_carries_the_clock_and_the_day_offset() {
        let state = PlanState { games: 3 };
        let status = state.status(
            20,
            Some("2026-11-24T23:30".to_string()),
            at(10, 10),
        );
        assert_eq!(status.stop_clock.as_deref(), Some("23:30"));
        assert_eq!(status.stop_days, Some(3));
        assert!(!status.deadline_passed);
    }

    #[test]
    fn the_budget_for_the_game_in_play_is_the_counter_plus_one() {
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
