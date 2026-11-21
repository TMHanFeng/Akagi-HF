//! Automatic "play again" after a game ends.
//!
//! Majsoul walks through a few screens when a game is over, and the walk
//! has to be clicked through. Measured on the live client (2026-11-20,
//! 繁體中文 client, three 半庄 finished and then walked through by hand
//! while the wire log was running):
//!
//! 1. the **終局** screen — the score board — with 確定 at the bottom
//!    right;
//! 2. the **禮物盒** panel — the gift box — with 確定 in the same place;
//! 3. the **道具** panel — the items the round paid out — with 確定 in the
//!    same place *and* the blue **再來一場** control right next to it;
//! 4. pressing 再來一場 does **not** ask for a game: it opens a second
//!    dialog — "要再進行一場 … 的匹配對局嗎?" — and it is the 確定 button
//!    *inside that dialog* that makes the client request the next match;
//! 5. pressing 確定 on the 道具 panel instead lands on the
//!    **match-preparation** screen — a rule card for the next game
//!    ("寶牌雙刃") with a big orange **匹配對局** button under it, which
//!    asks for a match on the spot, without a dialog.
//!
//! So there are four coordinates, all measured, all 16:9-normalised
//! against the game canvas:
//!
//! * 確定 — `(14.59, 8.37)`, the same on all three result screens;
//! * 再來一場 — `(12.23, 8.34)`, on the reward panel only;
//! * the dialog's 確定 — `(6.58, 6.71)`, in the middle of the screen;
//! * 匹配對局 — `(10.87, 7.10)`, on the match-preparation screen.
//!
//! And there is one thing the wire does *not* say. Dismissing the result
//! screens sends **nothing**: pressing 確定 on the three screens moved the
//! client three screens forward and produced zero frames on the uplink.
//! So the loop cannot wait for "the screen is gone" — it has to click
//! blind, on a bounded schedule, and use the messages that do come:
//!
//! * `.lq.Lobby.startUnifiedMatch` — the client asked for a match. The
//!   dialog's 確定 press landed. Stop.
//! * `.lq.Route.requestRouteChange` with `route_id: "route-3"` — the
//!   client went to the match-preparation route. That screen has a button
//!   of its own; press it and stop.
//! * a route change to the lobby (`.lq.Route.requestRouteChange` with
//!   `route_id: "route-4"`) — the lobby route was reloaded. Stop; the
//!   再來一場 control does not exist there.
//!
//! (`route-4` is weaker than it sounds and the measurement says so: the
//! client reloads the lobby route on its own schedule — it was seen once
//! *during* a game and once 3 m 38 s after the client had been left on
//! the match-preparation screen. It is a safety stop, not a step in the
//! plan. The result screens themselves never lead to the lobby: the
//! reward panel's 確定 goes to the match-preparation screen (measured),
//! so the 確定 coordinate can never be pressed on the lobby's own
//! furniture by this loop.)
//!
//! (`.lq.FastTest.confirmNewRound` is *not* one of them: it is the
//! readiness check that comes with a match, not the dismissal of the old
//! one's result screens. See `autoplay::lobby_watch`.)
//!
//! # The dialog is invisible on the wire
//!
//! This is the thing the loop got wrong once, and it is worth writing
//! down. The dialog that 再來一場 opens — "要再進行一場 … 的匹配對局嗎?"
//! — is **not** on the wire. Measured on the live client, with the whole
//! walk watched frame by frame (2026-11-20, 21:0x, this module's own
//! Chromium): pressing 再來一場 produced *no* message at all — no
//! request, no notification, **no route change** — and `route-3` never
//! appeared anywhere in the walk. The dialog's own 確定 press, on the
//! other hand, put `.lq.Lobby.startUnifiedMatch` on the wire in the same
//! millisecond as the click.
//!
//! So the dialog has to be answered *blind*, immediately after the press
//! that opens it. A loop that waits for a signal before pressing the
//! dialog's 確定 waits forever — and, worse, a loop that treats the
//! dialog's press as optional goes on to press 確定 behind the dialog,
//! which walks the client off the reward panel while the dialog is still
//! up.
//!
//! The walk is therefore a fixed order, not a script the wire drives:
//!
//! ```text
//! 每輪： 再來一場 ──> 對話框的確定 ──> 確定（往前走一屏）
//!                              │
//!                              └── startUnifiedMatch? ──> 開局
//! ```
//!
//! 再來一場 comes first for a reason: it is the control that asks for
//! the next game, and it only exists on the reward panel — the last
//! screen the walk visits. The dialog it opens is answered at once,
//! because nothing on the wire will ever say it is there. The cost of
//! putting 再來一場 first is one wasted press on the screens that do not
//! carry it. Measured, that press is inert: on the 終局 screen it lands
//! on the player's own tile wall, and in the lobby it lands in the gap
//! between 牌譜 and 倉庫. The dialog's 確定 is inert in the lobby too —
//! it lands on the character's hair.
//!
//! The one coordinate that is *not* inert in the lobby is 確定 itself:
//! measured on the live client, pressing it there sends the client to the
//! rank-match route and starts a game. That is why the loop caps the
//! number of 確定 presses (`max_dismiss_presses`) on top of watching the
//! route.
//!
//! The cap is what makes the 匹配對局 button reachable: once the 確定
//! budget is spent, the client stands on the match-preparation screen,
//! where that button asks for a match directly. So the loop still gets
//! into a game even when the 再來一場 control is gone — which is what
//! happens once the other three players leave.
//!
//! Coordinates are 16:9-normalised against the game canvas, the same
//! convention as `autoplay::majsoul::coords`. They are read against the
//! canvas' own rect on every press, so they survive a window resize: the
//! canvas scales with the viewport, and the UI inside it keeps its
//! relative position. Measured in this module's own Chromium (canvas
//! 1036 × 582 CSS px at y = 95, content 2560 × 1440): 確定 lands on
//! (944.7, 636.3), inside the gold 確定 plate; 再來一場 on (791.9,
//! 634.3), inside the blue 再來一場 plate; the dialog's 確定 on (426.1,
//! 528.9), inside the dialog's gold 確定.

use std::sync::Arc;
use std::time::Duration;

use tracing::{info, warn};

use crate::autoplay::cdp_input::{dispatch_click_shaped, evaluate_canvas_rect};
use crate::autoplay::context::AutoplayContext;
use crate::autoplay::lobby_watch::{LobbyTicket, LobbyWatch, SharedLobbyWatch};
use crate::config::RequeueConfig;

/// How often the loop looks at the wire while it waits. Small enough that
/// the extra press after the client answered is rare, large enough not to
/// spin.
const POLL_SLICE_MS: u32 = 100;

/// How often the loop may press 匹配對局 on its own.
///
/// The route change that tells the loop "the client is on the
/// match-preparation screen" is not proof that the button is on screen *yet*:
/// measured on the live client, the route changed while the client was still
/// rendering the gift box, and the press landed on the artwork behind it.
/// So the button gets a few more tries — but only a few, or the press budget
/// would be spent on a screen that is not there.
const MATCH_PRESS_LIMIT: u32 = 4;

/// Which control a candidate is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum Button {
    /// 再來一場 — opens the confirmation dialog.
    Queue,
    /// The 確定 inside the dialog 再來一場 opens — asks for the next game.
    DialogConfirm,
    /// 確定 — walks the client through the result screens.
    Confirm,
    /// 匹配對局 — asks for a match from the match-preparation screen.
    Match,
    /// A configured extra — a popup's own dismiss control.
    Extra,
}

/// One control the loop may press.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct Candidate {
    button: Button,
    point: [f64; 2],
}

/// What the loop ended up doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The client asked for a match — the next game is on its way.
    Queued,
    /// The client is back in the lobby, but no match was asked for.
    Lobby,
    /// The budget ran out without an answer.
    Budget,
    /// A press could not be delivered.
    Failed,
}

/// What the wire said while the loop waited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Signal {
    /// Nothing yet.
    None,
    /// The client asked for a match.
    Match,
    /// The client went to the match-preparation route.
    ///
    /// This is the screen with the big orange 匹配對局 button on it — the
    /// one the client reaches when 確定 is pressed on the reward panel.
    /// It is *not* a signal that a dialog is up: the dialog 再來一場
    /// opens says nothing on the wire at all (measured — no request, no
    /// notification, no route change), so the loop cannot wait for it and
    /// must press the dialog's 確定 straight after the press that opened
    /// it.
    MatchPrepared,
    /// The client reloaded the lobby route.
    Lobby,
}

/// Run the press sequence for one finished game.
///
/// Runs as its own task: the manager's plan loop is busy at this point and
/// the presses must not inherit that delay.
pub async fn run_requeue(ctx: Arc<AutoplayContext>, cfg: RequeueConfig, games_played: u32) {
    info!("requeue: game {games_played} ended — walking the result screens");

    if cfg.result_delay_ms > 0 {
        tokio::time::sleep(Duration::from_millis(cfg.result_delay_ms as u64)).await;
    }

    let outcome = drive(&ctx, &cfg).await;
    match outcome {
        Outcome::Queued => {
            info!("requeue: the client asked for a match — the next game is on its way")
        }
        Outcome::Lobby => info!(
            "requeue: the client went back to the lobby — no match was asked for \
             (the 再來一場 control is only there while the others stay)"
        ),
        Outcome::Budget => warn!(
            "requeue: the press budget ran out — the client was not put back \
             into a game"
        ),
        Outcome::Failed => warn!("requeue: a press could not be delivered — giving up"),
    }
}

/// The loop itself.
async fn drive(ctx: &Arc<AutoplayContext>, cfg: &RequeueConfig) -> Outcome {
    let watch = ctx.lobby_watch.clone();
    let ticket: LobbyTicket = watch.ticket();
    let budget = cfg.max_confirm_presses.max(1);
    let step = cfg.step_wait_ms.max(200);

    let mut presses = 0u32;
    let mut dismisses = 0u32;
    let mut match_presses = 0u32;
    let mut extra_at = 0usize;

    while presses < budget {
        // The client went back to the lobby on its own: the 再來一場 control
        // does not exist there and the 確定 coordinate belongs to the
        // lobby's own furniture. Stop before touching any of them.
        if watch.lobby_entered_since(ticket) {
            return Outcome::Lobby;
        }

        // ── 1. The queue control ────────────────────────────────────────
        //
        // 再來一場 only exists on the reward panel; on the screens before
        // it the press lands on empty space and does nothing (measured).
        // It is offered on *every* screen, because the reward panel is
        // the last one the walk visits and the dialog it opens is
        // answered right away.
        if let Some(queue) = cfg.queue_button {
            if !press(ctx, "再來一場", queue[0], queue[1], cfg).await {
                return Outcome::Failed;
            }
            presses += 1;
            if wait_for_signal(&watch, &ticket, step).await == Signal::Match {
                info!("requeue: the match request went out after {presses} press(es)");
                return Outcome::Queued;
            }

            // ── 2. The dialog's own 確定 ─────────────────────────────────
            //
            // This is the press that asks for the next game. It has to
            // come *immediately* after the queue press: the dialog the
            // queue press opens is invisible on the wire — no request,
            // no notification, no route change (measured) — so nothing
            // else can tell the loop that it is up, and anything pressed
            // in between would land on the panel *behind* the dialog and
            // walk the client away from it.
            //
            // If no dialog is up, this lands on empty space and does
            // nothing (measured).
            if !press(
                ctx,
                "對話框的確定",
                cfg.queue_dialog_confirm[0],
                cfg.queue_dialog_confirm[1],
                cfg,
            )
            .await
            {
                return Outcome::Failed;
            }
            presses += 1;
            match wait_for_signal(&watch, &ticket, cfg.queue_confirm_timeout_ms).await {
                Signal::Match => {
                    info!("requeue: the match request went out after {presses} press(es)");
                    return Outcome::Queued;
                }
                Signal::Lobby => return Outcome::Lobby,
                _ => {}
            }
        }

        // ── 3. The client's own popups ──────────────────────────────────
        //
        // Their dismiss controls, if the user measured any. One per
        // round; the walk carries on either way.
        if !cfg.extra_buttons.is_empty() {
            let extra = cfg.extra_buttons[extra_at % cfg.extra_buttons.len()];
            extra_at += 1;
            if !press(ctx, "extra", extra[0], extra[1], cfg).await {
                return Outcome::Failed;
            }
            presses += 1;
            if wait_for_signal(&watch, &ticket, step).await == Signal::Match {
                return Outcome::Queued;
            }
        }

        // ── 4. Walk one screen further ─────────────────────────────────
        //
        // 確定 is the control that walks the client from the score board
        // to the gift box to the reward panel — and, one more time, to
        // the match-preparation screen.
        let dismiss_allowed = cfg.max_dismiss_presses == 0 || dismisses < cfg.max_dismiss_presses;
        if dismiss_allowed {
            if !press(
                ctx,
                "確定",
                cfg.result_confirm[0],
                cfg.result_confirm[1],
                cfg,
            )
            .await
            {
                return Outcome::Failed;
            }
            presses += 1;
            dismisses += 1;
            match wait_for_signal(&watch, &ticket, step).await {
                Signal::Match => return Outcome::Queued,
                Signal::Lobby => return Outcome::Lobby,
                _ => {}
            }

            // The client may have walked all the way to the
            // match-preparation screen — and that one *is* on the wire:
            // the route change to it is a message, unlike the result
            // screens, which say nothing at all.
            //
            // That screen has a button of its own and asks for a match
            // directly. Press it and stop: the 再來一場 coordinate is
            // meaningless there, and pressing around on that screen is
            // how a walk turns into a mis-click.
            if watch.match_prepared_since(ticket) && match_presses < MATCH_PRESS_LIMIT {
                if let Some(m) = cfg.match_button {
                    if !press(ctx, "匹配對局", m[0], m[1], cfg).await {
                        return Outcome::Failed;
                    }
                    presses += 1;
                    match_presses += 1;
                    match wait_for_signal(&watch, &ticket, cfg.queue_confirm_timeout_ms).await {
                        Signal::Match => {
                            info!("requeue: the match request went out after {presses} press(es)");
                            return Outcome::Queued;
                        }
                        Signal::Lobby => return Outcome::Lobby,
                        _ => {}
                    }
                }
                // The route change is **not** proof that the button is on the
                // screen: the client can change route while it is still
                // rendering a result screen, and the button may be seconds
                // away. Measured on the live client — the route changed while
                // the gift box was still up, the press landed on the artwork
                // behind it, and the loop gave up with 23 of its 36 presses
                // unused. The client was still clickable afterwards.
                //
                // So: keep walking. The next round presses 再來一場 and the
                // dialog's 確定 again, and those are what ask for the next
                // game once the reward panel is up.
                continue;
            }
            continue;
        }

        // ── 5. Out of dismissals ───────────────────────────────────────
        //
        // The client has walked past every result screen and stands on
        // the match-preparation screen, which has a button of its own.
        if let Some(m) = cfg.match_button {
            if !press(ctx, "匹配對局", m[0], m[1], cfg).await {
                return Outcome::Failed;
            }
            presses += 1;
            match wait_for_signal(&watch, &ticket, cfg.queue_confirm_timeout_ms).await {
                Signal::Match => {
                    info!("requeue: the match request went out after {presses} press(es)");
                    return Outcome::Queued;
                }
                Signal::Lobby => return Outcome::Lobby,
                _ => {}
            }
            continue;
        }

        // Nothing left to press.
        break;
    }

    Outcome::Budget
}

/// The controls the loop prefers, in order.
///
/// 再來一場 comes first: it is the control that asks for the next game, and
/// it only exists on the last result screen. The dialog it opens is
/// answered right after it, and 確定 — which walks the client one screen
/// further — comes after that, so the queue press is never skipped by a
/// dismissal. The configured extras sit between the dialog and 確定: they
/// are the dismiss controls of the client's own popups, and a popup that
/// is up would swallow the 確定 press. 匹配對局 comes last — by the time
/// the client stands on the match-preparation screen the 再來一場 control
/// is gone, and that button is the only way left into a game.
///
/// The loop itself is a walk rather than a rotation — the wire decides
/// when a screen is ready — but the order above is the order it prefers.
#[allow(dead_code)]
fn candidates(cfg: &RequeueConfig) -> Vec<Candidate> {
    let mut out = Vec::with_capacity(4 + cfg.extra_buttons.len());
    if let Some(queue) = cfg.queue_button {
        out.push(Candidate {
            button: Button::Queue,
            point: queue,
        });
    }
    out.push(Candidate {
        button: Button::DialogConfirm,
        point: cfg.queue_dialog_confirm,
    });
    for extra in cfg.extra_buttons.iter().copied() {
        out.push(Candidate {
            button: Button::Extra,
            point: extra,
        });
    }
    out.push(Candidate {
        button: Button::Confirm,
        point: cfg.result_confirm,
    });
    if let Some(m) = cfg.match_button {
        out.push(Candidate {
            button: Button::Match,
            point: m,
        });
    }
    out
}

/// Watch the wire for up to `timeout_ms`.
///
/// Returns as soon as the client says something. The match request arrives
/// about two seconds after the click, so the wait ends early and the loop
/// does not press a screen it has already left.
async fn wait_for_signal(
    watch: &SharedLobbyWatch,
    ticket: &LobbyTicket,
    timeout_ms: u32,
) -> Signal {
    let mut waited = 0u32;
    while waited < timeout_ms {
        let slice = POLL_SLICE_MS.min(timeout_ms - waited);
        tokio::time::sleep(Duration::from_millis(slice as u64)).await;
        waited += slice;
        match signal_now(watch.as_ref(), ticket) {
            Signal::None => {}
            other => return other,
        }
    }
    signal_now(watch.as_ref(), ticket)
}

/// The strongest signal the wire is currently showing.
fn signal_now(watch: &LobbyWatch, ticket: &LobbyTicket) -> Signal {
    if watch.start_unified_match_since(*ticket) {
        Signal::Match
    } else if watch.match_prepared_since(*ticket) {
        Signal::MatchPrepared
    } else if watch.lobby_entered_since(*ticket) {
        Signal::Lobby
    } else {
        Signal::None
    }
}

/// Press one normalised point on the game canvas.
///
/// `name` is the control's name, for the log: the log line is the only
/// record of what the loop actually pressed, and the walk's whole
/// correctness rests on that order being the one the client expects.
async fn press(
    ctx: &Arc<AutoplayContext>,
    name: &str,
    x_norm: f64,
    y_norm: f64,
    cfg: &RequeueConfig,
) -> bool {
    let page_guard = ctx.page.read().await;
    let Some(page) = page_guard.as_ref().cloned() else {
        warn!("requeue: no page handle — cannot press");
        return false;
    };
    drop(page_guard);

    let Ok(rect) = evaluate_canvas_rect(&page).await else {
        warn!("requeue: could not resolve the canvas rect");
        return false;
    };
    let (px, py) = rect.pixel(x_norm, y_norm);
    if !rect.contains(px, py) {
        warn!("requeue: ({x_norm}, {y_norm}) lands outside the canvas rect — skipping");
        return false;
    }
    info!("requeue: pressing {name} at ({x_norm}, {y_norm}) — page ({px:.1}, {py:.1})");
    match dispatch_click_shaped(&page, px, py, cfg.hover_delay_ms, cfg.click_hold_ms, false).await {
        Ok(()) => true,
        Err(e) => {
            warn!("requeue: dispatch_click failed: {e:#}");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::autoplay::lobby_watch::LobbyWatch;
    use crate::config::RequeueConfig;

    /// 再來一場 first, the dialog's 確定 second, the configured extras
    /// third, 確定 fourth and 匹配對局 last: the queue press has to be
    /// offered to every screen before that screen is dismissed, and the
    /// dialog it opens has to be answered at once.
    #[test]
    fn the_controls_are_tried_in_order() {
        let mut cfg = RequeueConfig::default();
        cfg.extra_buttons = vec![[10.0, 6.0], [12.5, 7.5]];
        let cands = candidates(&cfg);
        assert_eq!(cands.len(), 6);
        assert_eq!(cands[0].button, Button::Queue);
        assert_eq!(cands[0].point, cfg.queue_button.unwrap());
        assert_eq!(cands[1].button, Button::DialogConfirm);
        assert_eq!(cands[1].point, cfg.queue_dialog_confirm);
        assert_eq!(cands[2].button, Button::Extra);
        assert_eq!(cands[2].point, [10.0, 6.0]);
        assert_eq!(cands[3].button, Button::Extra);
        assert_eq!(cands[3].point, [12.5, 7.5]);
        assert_eq!(cands[4].button, Button::Confirm);
        assert_eq!(cands[4].point, cfg.result_confirm);
        assert_eq!(cands[5].button, Button::Match);
        assert_eq!(cands[5].point, cfg.match_button.unwrap());
    }

    /// The queue control is offered on *every* screen, not on the even ones
    /// only: in the rotation it comes before 確定, so the screen that
    /// carries 再來一場 is never dismissed before the press was made.
    #[test]
    fn the_queue_control_is_offered_before_every_dismissal() {
        let cfg = RequeueConfig::default();
        let cands = candidates(&cfg);
        let queue_at = cands
            .iter()
            .position(|c| c.button == Button::Queue)
            .expect("the queue control is configured");
        let confirm_at = cands
            .iter()
            .position(|c| c.button == Button::Confirm)
            .expect("the confirm control is configured");
        assert!(queue_at < confirm_at);
    }

    /// The dialog's 確定 comes right after the 再來一場 press that opens
    /// it — nothing may come between them.
    #[test]
    fn the_dialog_follows_the_queue_press() {
        let cfg = RequeueConfig::default();
        let cands = candidates(&cfg);
        let queue_at = cands
            .iter()
            .position(|c| c.button == Button::Queue)
            .expect("the queue control is configured");
        let dialog_at = cands
            .iter()
            .position(|c| c.button == Button::DialogConfirm)
            .expect("the dialog control is configured");
        assert_eq!(dialog_at, queue_at + 1);
    }

    /// Without a queue control the loop is the dialog and the confirm
    /// control alone.
    #[test]
    fn without_a_queue_button_the_dialog_is_pressed_too() {
        let mut cfg = RequeueConfig::default();
        cfg.queue_button = None;
        let cands = candidates(&cfg);
        assert_eq!(cands.len(), 3);
        assert_eq!(cands[0].button, Button::DialogConfirm);
        assert_eq!(cands[0].point, cfg.queue_dialog_confirm);
        assert_eq!(cands[1].button, Button::Confirm);
        assert_eq!(cands[1].point, cfg.result_confirm);
        assert_eq!(cands[2].button, Button::Match);
    }
    /// The measured defaults: the 確定 control at the bottom right, the
    /// 再來一場 control next to it, the dialog's 確定 in the middle and the
    /// 匹配對局 control on the match-preparation screen.
    #[test]
    fn the_defaults_are_the_measured_coordinates() {
        let cfg = RequeueConfig::default();
        assert_eq!(cfg.result_confirm, [14.59, 8.37]);
        assert_eq!(cfg.queue_button, Some([12.23, 8.34]));
        assert_eq!(cfg.queue_dialog_confirm, [6.58, 6.71]);
        assert_eq!(cfg.match_button, Some([10.87, 7.10]));
    }

    /// The 確定 coordinate is the dangerous one in the lobby, so it may
    /// only be pressed a bounded number of times — but the bound has to
    /// outlast the result screens' animations, or one wasted press
    /// strands the walk before the reward panel.
    #[test]
    fn the_dismiss_budget_outlasts_the_result_screens() {
        let cfg = RequeueConfig::default();
        assert!(cfg.max_dismiss_presses >= 3, "three result screens");
        assert!(
            cfg.max_dismiss_presses > 3,
            "room for an animation that eats one press"
        );
    }

    /// The press budget has to cover the longest walk the client can take:
    /// three rounds of 再來一場 / 對話框確定 / 確定 to reach the reward
    /// panel, the round that uses the dialog, and — if 再來一場 is gone —
    /// the 確定 presses to the match-preparation screen and the 匹配對局
    /// press on it.
    #[test]
    fn the_press_budget_covers_the_longest_walk() {
        let cfg = RequeueConfig::default();
        let rounds = 3 + 1; // to the reward panel, then the round that uses it
        let per_round = 3; // 再來一場, 對話框確定, 確定
        assert!(cfg.max_confirm_presses >= rounds * per_round);
        let fallback = 4 * per_round + 1; // 匹配對局 on top
        assert!(cfg.max_confirm_presses >= fallback);
    }

    /// The measured defaults after the *third* walk — the first one that
    /// ran the loop for real: three presses per round, and enough rounds
    /// to cover the three result screens plus the round that uses the
    /// dialog, with slack for the screens' animations.
    ///
    /// The first live run needed 17 presses: three of its five 確定
    /// presses landed on an animating screen and did nothing, so the
    /// budget has to be sized for that, not for the shortest walk.
    #[test]
    fn the_default_budget_leaves_room_for_the_animations() {
        let cfg = RequeueConfig::default();
        assert!(cfg.max_confirm_presses >= 24, "a walk that loses presses");
        assert!(cfg.max_dismiss_presses >= 6, "three result screens, twice over");
        assert!(cfg.step_wait_ms >= 1500, "the result screens animate");
        assert!(
            cfg.queue_confirm_timeout_ms >= 1000,
            "the match request goes out in the same millisecond as the click"
        );
    }

    /// A match request outranks everything else: it is the one answer that
    /// means "stop, the next game is on its way".
    #[test]
    fn the_match_signal_wins_over_the_others() {
        let w = LobbyWatch::default();
        let t = w.ticket();
        w.note_match_prepared();
        w.note_lobby_entered();
        assert_eq!(signal_now(&w, &t), Signal::MatchPrepared);
        w.note_start_unified_match();
        assert_eq!(signal_now(&w, &t), Signal::Match);
    }

    /// The lobby is the last word when no match is on its way.
    #[test]
    fn the_lobby_signal_is_the_last_resort() {
        let w = LobbyWatch::default();
        let t = w.ticket();
        assert_eq!(signal_now(&w, &t), Signal::None);
        w.note_lobby_entered();
        assert_eq!(signal_now(&w, &t), Signal::Lobby);
    }
}
