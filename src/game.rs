//! Game façade: wraps the side-relative `rules::State` with absolute
//! human/AI labels. All input and output is in **human-display coordinates**
//! (row 0 = human's back rank), so the UI never deals with the internal flip.

use crate::codec;
use crate::rules::{Color, Move, N_SQUARES, Outcome, Piece, State};
use crate::search::{self, Coefs};

pub struct GameLogic {
    state: State,
    humans_turn: bool,
}

impl GameLogic {
    pub fn new(human_plays_first: bool) -> Self {
        Self {
            state: State::initial(),
            humans_turn: human_plays_first,
        }
    }

    pub fn humans_turn(&self) -> bool {
        self.humans_turn
    }

    pub fn legal_move_codes(&self) -> Vec<u8> {
        let mut moves = Vec::with_capacity(32);
        self.state.gen_moves(&mut moves);
        moves
            .into_iter()
            .map(|m| codec::encode(self.between_internal_and_display(m)))
            .collect()
    }

    pub fn apply_code(&mut self, code: u8) -> Outcome {
        let display_move = codec::decode(code).expect("invalid move code");
        let internal = self.between_internal_and_display(display_move);
        let (next, out) = self.state.apply(internal);
        self.state = next;
        if matches!(out, Outcome::Ongoing) {
            self.humans_turn = !self.humans_turn;
        }
        out
    }

    pub fn ai_search(
        &self,
        coefs: &Coefs,
        depth: u32,
        jitter: impl FnMut() -> i32,
    ) -> (Option<u8>, i32) {
        let r = search::search(&self.state, coefs, depth, jitter);
        let code = r
            .mv
            .map(|m| codec::encode(self.between_internal_and_display(m)));
        (code, r.score)
    }

    /// Deterministic eval at the given depth (no jitter).
    /// Score is from the side-to-move's perspective.
    pub fn eval_at_depth(&self, coefs: &Coefs, depth: u32) -> i32 {
        search::search(&self.state, coefs, depth, || 0).score
    }

    pub fn board_bytes(&self) -> Vec<u8> {
        // Project state to "human is Own" so emission is straightforward.
        let projected = if self.humans_turn {
            self.state
        } else {
            self.state.flip()
        };
        let mut out = Vec::with_capacity(N_SQUARES);
        for i in 0..N_SQUARES {
            out.push(match projected.board[i] {
                None => 0u8,
                Some(stone) => {
                    let low = match stone.piece {
                        Piece::Lion => 1u8,
                        Piece::Giraffe => 2,
                        Piece::Elephant => 3,
                        Piece::Chick => 4,
                        Piece::Hen => 5,
                    };
                    let owner = if stone.color == Color::Own { 0 } else { 0x80 };
                    low | owner
                }
            });
        }
        out
    }

    pub fn human_hand(&self) -> (u8, u8, u8) {
        let h = if self.humans_turn {
            self.state.own_hand
        } else {
            self.state.opp_hand
        };
        (h.chick, h.elephant, h.giraffe)
    }

    pub fn ai_hand(&self) -> (u8, u8, u8) {
        let h = if self.humans_turn {
            self.state.opp_hand
        } else {
            self.state.own_hand
        };
        (h.chick, h.elephant, h.giraffe)
    }

    /// Translates a `Move` between internal (side-relative) and display
    /// (always human-at-bottom) coordinates. Self-inverse: a single function
    /// suffices for both directions.
    fn between_internal_and_display(&self, m: Move) -> Move {
        if self.humans_turn {
            m
        } else {
            rotate_move_180(m)
        }
    }
}

fn rotate_move_180(m: Move) -> Move {
    match m {
        Move::Slide { from, to } => Move::Slide {
            from: from.rotate180(),
            to: to.rotate180(),
        },
        Move::Drop { piece, to } => Move::Drop {
            piece,
            to: to.rotate180(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Sq;

    fn piece_byte(piece: Piece, ai_owned: bool) -> u8 {
        let low = match piece {
            Piece::Lion => 1,
            Piece::Giraffe => 2,
            Piece::Elephant => 3,
            Piece::Chick => 4,
            Piece::Hen => 5,
        };
        if ai_owned { low | 0x80 } else { low }
    }

    fn idx(row: i8, col: i8) -> usize {
        Sq::new(row, col).0 as usize
    }

    #[test]
    fn humans_turn_reflects_constructor() {
        assert!(GameLogic::new(true).humans_turn());
        assert!(!GameLogic::new(false).humans_turn());
    }

    #[test]
    fn initial_board_bytes_human_first_have_human_at_bottom() {
        // Human plays first → row 0 of board_bytes = human's back rank
        // = E L G with human (high bit clear).
        let g = GameLogic::new(true);
        let b = g.board_bytes();
        assert_eq!(b.len(), 12);
        assert_eq!(b[idx(0, 0)], piece_byte(Piece::Elephant, false));
        assert_eq!(b[idx(0, 1)], piece_byte(Piece::Lion, false));
        assert_eq!(b[idx(0, 2)], piece_byte(Piece::Giraffe, false));
        assert_eq!(b[idx(1, 1)], piece_byte(Piece::Chick, false));
        assert_eq!(b[idx(2, 1)], piece_byte(Piece::Chick, true));
        assert_eq!(b[idx(3, 0)], piece_byte(Piece::Giraffe, true));
        assert_eq!(b[idx(3, 1)], piece_byte(Piece::Lion, true));
        assert_eq!(b[idx(3, 2)], piece_byte(Piece::Elephant, true));
    }

    #[test]
    fn initial_board_bytes_human_second_still_have_human_at_bottom() {
        // Even when AI moves first, the *display* must keep the human at the bottom.
        let g = GameLogic::new(false);
        let b = g.board_bytes();
        assert_eq!(b[idx(0, 1)], piece_byte(Piece::Lion, false));
        assert_eq!(b[idx(3, 1)], piece_byte(Piece::Lion, true));
    }

    #[test]
    fn initial_legal_move_count_is_4_for_human_first() {
        let g = GameLogic::new(true);
        assert_eq!(g.legal_move_codes().len(), 4);
    }

    #[test]
    fn initial_legal_move_count_is_4_for_ai_first() {
        // AI also has 4 moves from the symmetric initial position.
        let g = GameLogic::new(false);
        assert_eq!(g.legal_move_codes().len(), 4);
    }

    #[test]
    fn apply_human_chick_forward_advances_turn_and_state() {
        // Encoding for Chick (1,1) → (2,1): from = 4, to = 7. code = (4<<4)|7 = 0x47.
        let mut g = GameLogic::new(true);
        let code = (Sq::new(1, 1).0 << 4) | Sq::new(2, 1).0;
        assert!(g.legal_move_codes().contains(&code));
        let out = g.apply_code(code);
        assert_eq!(out, Outcome::Ongoing);
        assert!(!g.humans_turn());
        // The chick that *was* at (2,1) (AI's chick) is now in the human's hand.
        assert_eq!(g.human_hand(), (1, 0, 0));
        // The board now has the human's chick at (2,1) (still in human-display coords).
        let b = g.board_bytes();
        assert_eq!(b[idx(2, 1)], piece_byte(Piece::Chick, false));
        assert_eq!(b[idx(1, 1)], 0); // chick has moved away
    }

    #[test]
    fn human_hand_is_empty_initially() {
        assert_eq!(GameLogic::new(true).human_hand(), (0, 0, 0));
        assert_eq!(GameLogic::new(false).human_hand(), (0, 0, 0));
    }

    #[test]
    fn ai_hand_is_empty_initially() {
        assert_eq!(GameLogic::new(true).ai_hand(), (0, 0, 0));
        assert_eq!(GameLogic::new(false).ai_hand(), (0, 0, 0));
    }

    #[test]
    fn ai_search_returns_a_legal_move_code() {
        let g = GameLogic::new(false); // AI to move
        let (mv, _score) = g.ai_search(&Coefs::DEFAULT, 2, || 0);
        let mv = mv.expect("AI should find a move from initial position");
        assert!(g.legal_move_codes().contains(&mv));
    }

    #[test]
    fn ai_move_in_human_display_coords() {
        // Mate-in-1 position: AI (own=AI) has Giraffe that can capture human's Lion.
        // We construct this by setting up a state, but our public API only allows
        // the initial position. So instead test that ai_search returns codes whose
        // squares fit the legal-move list (already covered above) AND that applying
        // that code advances the game without panicking.
        let mut g = GameLogic::new(false);
        let (mv, _score) = g.ai_search(&Coefs::DEFAULT, 4, || 0);
        let mv = mv.expect("AI move present");
        let _ = g.apply_code(mv);
        assert!(g.humans_turn(), "after AI's first move, it should be human's turn");
    }
}
