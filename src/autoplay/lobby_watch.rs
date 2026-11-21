//! What the wire says about the client leaving a finished game.
//!
//! The "play again" loop cannot see the screen: Majsoul renders it to a
//! canvas, and the canvas is only readable as pixels — the autoplay has no
//! pixel reader. So the loop cannot ask "is the score board up yet?" and
//! cannot ask "did my click dismiss it?". The wire can, but only for the
//! messages the client actually sends, and what it sends is not what the
//! names suggest.
//!
//! Measured on the live client (2026-11-20, 繁體中文 client, two 半庄
//! finished and then walked through by hand while the wire log was running):
//!
//! * **Dismissing the result screens sends nothing.** Pressing 確定 on the
//!   終局 screen, then on the 禮物盒 screen, then on the 道具 screen moved
//!   the client three screens forward and produced *zero* frames on the
//!   uplink — no request, no notification. The same was true of the
//!   frames AkagiMS's own autoplay produced when it walked the same
//!   screens.
//! * **`.lq.FastTest.confirmNewRound` is not that.** It is the client
//!   confirming readiness for the *next* game: in the captured session it
//!   landed 23 ms after a click on the bottom-right 確定 and the first 局
//!   started in the same millisecond, 2 m 51 s after the match had been
//!   found. It says "the new round is under way", not "the old one is
//!   over".
//! * **`.lq.Lobby.startUnifiedMatch` is the request for a match** — the
//!   proof that a queue press landed.
//! * **`.lq.Route.requestRouteChange` says where the client goes next, and
//!   the route id is the whole message.** Two of them were measured on the
//!   live client:
//!
//!   * `route_id: "route-3"` — the client went to the match-preparation
//!     route: the screen that shows the rule card for the next game
//!     ("寶牌雙刃") with a big orange 匹配對局 button under it.
//!
//!     It is **not** the 再來一場 dialog. The dialog says nothing on the
//!     wire at all — measured on the live client with the whole walk
//!     watched frame by frame (2026-11-20, 21:0x): pressing 再來一場
//!     produced no request, no notification and no route change, and
//!     `route-3` never appeared anywhere in the walk. A loop that waits
//!     for this signal before pressing the dialog's 確定 waits forever.
//!   * `route_id: "route-4"` — the lobby route was (re)loaded.
//!
//!   The obvious candidate for the lobby signal, `.lq.Lobby.fetchAccountInfo`,
//!   is **not** it. Measured: it fires about 24 seconds after a game ends
//!   — while the result screens are still up — because the lobby side
//!   refreshes the account once the rank has changed. The very same
//!   message, in the very same shape, was seen at the moment the client
//!   really did return to the lobby, so the method alone cannot tell the
//!   two apart.
//!
//!   `route-4` is weaker than it looks, and the measurement says so: it
//!   fires when the lobby route is reloaded, which is **not** the same
//!   thing as "the client is on the lobby screen". It was seen once
//!   *during* a game and once 3 minutes 38 seconds after the client had
//!   been left sitting on the match-preparation screen — the client
//!   reloads the lobby route on its own schedule. The result screens the
//!   client walks after a game never include the lobby at all (the
//!   reward panel's 確定 goes to the match-preparation screen, not to the
//!   lobby — measured), so the loop keeps this signal as a safety stop
//!   rather than as a step in its plan, and leans on the press budget for
//!   the actual walk.
//!
//! So the loop has two usable signals and one hole:
//!
//! * `start_unified_match` — the queue press worked. Success.
//! * `match_prepared` — the client walked to the match-preparation
//!   screen. Press the 匹配對局 button on it.
//! * `lobby_entered` — the lobby route was reloaded. Stop; the 再來一場
//!   control does not exist there.
//! * the hole — there is no "one more screen dismissed" signal, so the
//!   loop cannot wait for one. It presses the result screens on a bounded
//!   schedule instead, and leans on the two signals above to know when to
//!   stop.
//!
//! The hole is wider than it looks: the 再來一場 dialog is invisible on
//! the wire too, so the loop cannot wait for that either. The dialog's
//! 確定 is pressed blind, right after the press that opens the dialog.
//!
//! A counter rather than a timestamp, for the same reason: the question is
//! "did *another* one happen since my ticket", which a monotonic count
//! answers without comparing clocks.

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

/// Counts the lobby-bound requests the client has sent.
#[derive(Debug, Default)]
pub struct LobbyWatch {
    /// `.lq.FastTest.confirmNewRound` — the client confirmed the *new*
    /// round. It arrives when a match starts, not when the old one ends.
    confirm_new_round: AtomicU64,
    /// `.lq.Lobby.startUnifiedMatch` — the client asked to enter a match.
    start_unified_match: AtomicU64,
    /// The client changed route to the match-preparation route — it is
    /// preparing a match, and the screen carries a 匹配對局 button of its
    /// own.
    match_prepared: AtomicU64,
    /// The client reloaded the lobby route.
    lobby_entered: AtomicU64,
}

/// A snapshot of the counts, taken before a click.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LobbyTicket {
    confirm_new_round: u64,
    start_unified_match: u64,
    match_prepared: u64,
    lobby_entered: u64,
}

impl LobbyWatch {
    /// The bridge saw the client confirm the new round.
    pub fn note_confirm_new_round(&self) {
        self.confirm_new_round.fetch_add(1, Ordering::Relaxed);
    }

    /// The bridge saw the client ask for a match.
    pub fn note_start_unified_match(&self) {
        self.start_unified_match.fetch_add(1, Ordering::Relaxed);
    }

    /// The bridge saw the client change route to the match-preparation
    /// route.
    pub fn note_match_prepared(&self) {
        self.match_prepared.fetch_add(1, Ordering::Relaxed);
    }

    /// The bridge saw the client reload the lobby route.
    pub fn note_lobby_entered(&self) {
        self.lobby_entered.fetch_add(1, Ordering::Relaxed);
    }

    /// Snapshot to compare against later.
    pub fn ticket(&self) -> LobbyTicket {
        LobbyTicket {
            confirm_new_round: self.confirm_new_round.load(Ordering::Relaxed),
            start_unified_match: self.start_unified_match.load(Ordering::Relaxed),
            match_prepared: self.match_prepared.load(Ordering::Relaxed),
            lobby_entered: self.lobby_entered.load(Ordering::Relaxed),
        }
    }

    /// Whether the client confirmed the new round since `ticket` was taken.
    pub fn confirm_new_round_since(&self, ticket: LobbyTicket) -> bool {
        self.confirm_new_round.load(Ordering::Relaxed) != ticket.confirm_new_round
    }

    /// Whether the client asked for a match since `ticket` was taken.
    pub fn start_unified_match_since(&self, ticket: LobbyTicket) -> bool {
        self.start_unified_match.load(Ordering::Relaxed) != ticket.start_unified_match
    }

    /// Whether the client changed route to the match-preparation route
    /// since `ticket` was taken.
    pub fn match_prepared_since(&self, ticket: LobbyTicket) -> bool {
        self.match_prepared.load(Ordering::Relaxed) != ticket.match_prepared
    }

    /// Whether the client reloaded the lobby route since `ticket` was
    /// taken.
    pub fn lobby_entered_since(&self, ticket: LobbyTicket) -> bool {
        self.lobby_entered.load(Ordering::Relaxed) != ticket.lobby_entered
    }
}

/// Shared between the bridge (which counts) and the requeue loop (which
/// checks). `Relaxed` ordering throughout: the only thing that matters is
/// that the count eventually differs, and every observation is separated
/// from the write by a real timer wait.
pub type SharedLobbyWatch = Arc<LobbyWatch>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ticket_only_moves_when_the_client_acts() {
        let w = LobbyWatch::default();
        let t = w.ticket();
        assert!(!w.confirm_new_round_since(t));
        assert!(!w.start_unified_match_since(t));
        assert!(!w.match_prepared_since(t));
        assert!(!w.lobby_entered_since(t));

        w.note_confirm_new_round();
        assert!(w.confirm_new_round_since(t));
        assert!(!w.start_unified_match_since(t));
        assert!(!w.match_prepared_since(t));
        assert!(!w.lobby_entered_since(t));

        let t2 = w.ticket();
        assert!(!w.confirm_new_round_since(t2));
        assert!(!w.start_unified_match_since(t2));
        assert!(!w.match_prepared_since(t2));
        assert!(!w.lobby_entered_since(t2));
        w.note_start_unified_match();
        assert!(w.start_unified_match_since(t2));
        assert!(!w.confirm_new_round_since(t2));
        assert!(!w.match_prepared_since(t2));
        assert!(!w.lobby_entered_since(t2));
    }

    /// The lobby signal is its own counter: a match request does not
    /// imply that the lobby was loaded, and the other way round.
    #[test]
    fn the_lobby_signal_is_independent() {
        let w = LobbyWatch::default();
        let t = w.ticket();
        w.note_lobby_entered();
        assert!(w.lobby_entered_since(t));
        assert!(!w.start_unified_match_since(t));
        assert!(!w.confirm_new_round_since(t));
        assert!(!w.match_prepared_since(t));

        let t2 = w.ticket();
        w.note_start_unified_match();
        assert!(w.start_unified_match_since(t2));
        assert!(!w.lobby_entered_since(t2));
    }

    /// The match-preparation signal is its own counter too: the client
    /// going to the match-preparation route says the 再來一場 dialog is
    /// up, which is a different statement from "a match was asked for" and
    /// from "the lobby was reloaded".
    #[test]
    fn the_match_preparation_signal_is_independent() {
        let w = LobbyWatch::default();
        let t = w.ticket();
        w.note_match_prepared();
        assert!(w.match_prepared_since(t));
        assert!(!w.start_unified_match_since(t));
        assert!(!w.lobby_entered_since(t));

        let t2 = w.ticket();
        w.note_start_unified_match();
        assert!(w.start_unified_match_since(t2));
        assert!(!w.match_prepared_since(t2));
    }
}
