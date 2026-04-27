//! Negamax + alpha-beta search with MVV move ordering.
//!
//! Side-to-move perspective: positive scores favour the side to move.
//! Mate scores are `±(MATE - ply_from_root)` so faster wins / slower
//! losses are preferred.

use crate::rules::{Color, Move, N_SQUARES, Outcome, Piece, State};

pub const MATE: i32 = 10_000;

/// Material coefficients used in the leaf evaluation. Lions are zero —
/// their capture ends the game and is scored as ±MATE separately.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Coefs {
    pub chick: i32,
    pub elephant: i32,
    pub giraffe: i32,
    pub hen: i32,
}

impl Coefs {
    pub const DEFAULT: Coefs = Coefs { chick: 4, elephant: 6, giraffe: 7, hen: 9 };

    fn value(&self, p: Piece) -> i32 {
        match p {
            Piece::Chick => self.chick,
            Piece::Elephant => self.elephant,
            Piece::Giraffe => self.giraffe,
            Piece::Hen => self.hen,
            Piece::Lion => 0,
        }
    }
}

impl Default for Coefs {
    fn default() -> Self {
        Coefs::DEFAULT
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SearchResult {
    pub mv: Option<Move>,
    pub score: i32,
}

/// Capture-ordering value. Lions sort above any other victim.
fn victim_order_value(coefs: &Coefs, p: Piece) -> i32 {
    match p {
        Piece::Lion => 1000,
        other => coefs.value(other),
    }
}

pub fn eval_leaf(s: &State, coefs: &Coefs) -> i32 {
    let mut score = 0;
    for i in 0..N_SQUARES {
        if let Some(stone) = s.board[i] {
            let v = coefs.value(stone.piece);
            score += if stone.color == Color::Own { v } else { -v };
        }
    }
    score += s.own_hand.chick as i32 * coefs.chick;
    score += s.own_hand.elephant as i32 * coefs.elephant;
    score += s.own_hand.giraffe as i32 * coefs.giraffe;
    score -= s.opp_hand.chick as i32 * coefs.chick;
    score -= s.opp_hand.elephant as i32 * coefs.elephant;
    score -= s.opp_hand.giraffe as i32 * coefs.giraffe;
    score
}

/// Stable sort: captures (highest victim value first) before quiet moves.
pub fn order_moves(s: &State, coefs: &Coefs, moves: &mut [Move]) {
    moves.sort_by_key(|m| {
        let v = match *m {
            Move::Slide { to, .. } => match s.at(to) {
                Some(stone) => victim_order_value(coefs, stone.piece),
                None => 0,
            },
            Move::Drop { .. } => 0,
        };
        -v
    });
}

fn negamax(s: &State, coefs: &Coefs, depth: u32, ply: u32, mut alpha: i32, beta: i32) -> i32 {
    if let Outcome::Try = s.immediate_outcome() {
        return -(MATE - ply as i32);
    }
    if depth == 0 {
        return eval_leaf(s, coefs);
    }
    let mut moves = Vec::with_capacity(32);
    s.gen_moves(&mut moves);
    if moves.is_empty() {
        return -(MATE - ply as i32);
    }
    order_moves(s, coefs, &mut moves);
    let mut best = i32::MIN;
    for m in moves {
        let (next, out) = s.apply(m);
        let score = match out {
            Outcome::LionCaptured | Outcome::Try => MATE - (ply as i32 + 1),
            Outcome::Ongoing => -negamax(&next, coefs, depth - 1, ply + 1, -beta, -alpha),
        };
        if score > best {
            best = score;
        }
        if best > alpha {
            alpha = best;
        }
        if alpha >= beta {
            break;
        }
    }
    best
}

/// Run a deterministic search at every depth from 1 to `max_depth` and
/// return the score reported at each. Used to surface how the engine's
/// evaluation evolves with depth in the move log.
pub fn iterative_evals(s: &State, coefs: &Coefs, max_depth: u32) -> Vec<i32> {
    (1..=max_depth)
        .map(|d| search(s, coefs, d, || 0).score)
        .collect()
}

/// Search the position to `depth` half-moves and return the best root move.
///
/// `jitter` is called once per candidate root move to perturb its score
/// for *selection only*; alpha-beta pruning continues to use raw scores
/// so the search remains correct. Pass `|| 0` for deterministic play.
pub fn search(
    s: &State,
    coefs: &Coefs,
    depth: u32,
    mut jitter: impl FnMut() -> i32,
) -> SearchResult {
    if let Outcome::Try = s.immediate_outcome() {
        return SearchResult { mv: None, score: -MATE };
    }
    let mut moves = Vec::with_capacity(32);
    s.gen_moves(&mut moves);
    if moves.is_empty() {
        return SearchResult { mv: None, score: -MATE };
    }
    order_moves(s, coefs, &mut moves);
    let mut best_mv = moves[0];
    let mut best_score = i32::MIN;
    let mut best_jittered = i32::MIN;
    let mut alpha = -MATE - 1;
    let beta = MATE + 1;
    for m in moves {
        let (next, out) = s.apply(m);
        let score = match out {
            Outcome::LionCaptured | Outcome::Try => MATE - 1,
            Outcome::Ongoing => -negamax(&next, coefs, depth.saturating_sub(1), 1, -beta, -alpha),
        };
        let jittered = score.saturating_add(jitter());
        if jittered > best_jittered {
            best_jittered = jittered;
            best_score = score;
            best_mv = m;
        }
        // Pruning uses raw scores — randomness must not affect search correctness.
        if score > alpha {
            alpha = score;
        }
    }
    SearchResult { mv: Some(best_mv), score: best_score }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::{Color, Hand, Sq, State, Stone};

    const C: Coefs = Coefs::DEFAULT;

    fn empty_state() -> State {
        State {
            board: [None; crate::rules::N_SQUARES],
            own_hand: Hand::default(),
            opp_hand: Hand::default(),
        }
    }

    fn place(s: &mut State, sq: Sq, piece: Piece, color: Color) {
        s.board[sq.0 as usize] = Some(Stone { piece, color });
    }

    #[test]
    fn eval_initial_is_symmetric() {
        // Material balanced regardless of coefficients.
        assert_eq!(eval_leaf(&State::initial(), &C), 0);
    }

    #[test]
    fn eval_lion_has_no_material_value() {
        let mut s = empty_state();
        place(&mut s, Sq::new(0, 0), Piece::Lion, Color::Own);
        place(&mut s, Sq::new(3, 0), Piece::Lion, Color::Opp);
        assert_eq!(eval_leaf(&s, &C), 0);
    }

    #[test]
    fn eval_chick_uses_chick_coef() {
        let mut s = empty_state();
        place(&mut s, Sq::new(1, 1), Piece::Chick, Color::Own);
        assert_eq!(eval_leaf(&s, &C), C.chick);
    }

    #[test]
    fn eval_elephant_and_giraffe_use_their_coefs() {
        let mut s = empty_state();
        place(&mut s, Sq::new(0, 0), Piece::Elephant, Color::Own);
        place(&mut s, Sq::new(0, 2), Piece::Giraffe, Color::Own);
        assert_eq!(eval_leaf(&s, &C), C.elephant + C.giraffe);
    }

    #[test]
    fn eval_hen_uses_hen_coef() {
        let mut s = empty_state();
        place(&mut s, Sq::new(2, 1), Piece::Hen, Color::Own);
        assert_eq!(eval_leaf(&s, &C), C.hen);
    }

    #[test]
    fn eval_opp_pieces_count_negative() {
        let mut s = empty_state();
        place(&mut s, Sq::new(2, 1), Piece::Chick, Color::Opp);
        assert_eq!(eval_leaf(&s, &C), -C.chick);
    }

    #[test]
    fn eval_hand_pieces_count_same_as_board() {
        let mut s = empty_state();
        s.own_hand.chick = 2;
        s.own_hand.elephant = 1;
        s.opp_hand.giraffe = 1;
        // own: 2*chick + 1*elephant; opp: 1*giraffe.
        let expected = 2 * C.chick + C.elephant - C.giraffe;
        assert_eq!(eval_leaf(&s, &C), expected);
    }

    #[test]
    fn eval_respects_custom_coefs() {
        let mut s = empty_state();
        place(&mut s, Sq::new(1, 1), Piece::Chick, Color::Own);
        let custom = Coefs { chick: 99, ..C };
        assert_eq!(eval_leaf(&s, &custom), 99);
    }

    /// Position: Own's Giraffe at (2,1) can capture Opp's Lion at (3,1).
    fn mate_in_one_position() -> State {
        let mut s = empty_state();
        place(&mut s, Sq::new(0, 1), Piece::Lion, Color::Own);
        place(&mut s, Sq::new(3, 1), Piece::Lion, Color::Opp);
        place(&mut s, Sq::new(2, 1), Piece::Giraffe, Color::Own);
        s
    }

    #[test]
    fn search_finds_mate_in_one_at_depth_1() {
        let s = mate_in_one_position();
        let r = search(&s, &C, 1, || 0);
        assert_eq!(r.score, MATE - 1);
        assert_eq!(
            r.mv,
            Some(Move::Slide {
                from: Sq::new(2, 1),
                to: Sq::new(3, 1),
            })
        );
    }

    #[test]
    fn search_still_finds_mate_in_one_at_depth_4() {
        let s = mate_in_one_position();
        let r = search(&s, &C, 4, || 0);
        assert_eq!(r.score, MATE - 1);
    }

    #[test]
    fn search_prefers_faster_mate() {
        // Direct lion capture available at root; deeper search must still pick it.
        let mut s = empty_state();
        place(&mut s, Sq::new(0, 0), Piece::Lion, Color::Own);
        place(&mut s, Sq::new(3, 0), Piece::Lion, Color::Opp);
        place(&mut s, Sq::new(3, 1), Piece::Giraffe, Color::Own);
        place(&mut s, Sq::new(2, 2), Piece::Chick, Color::Own);
        let r = search(&s, &C, 4, || 0);
        assert_eq!(r.score, MATE - 1);
    }

    #[test]
    fn search_jitter_can_alter_root_selection() {
        // With four equally-scored root moves, biasing the jitter on the
        // second candidate makes it win where the first would otherwise.
        let s = State::initial();
        let r0 = search(&s, &C, 1, || 0);
        let mut counter = 0;
        let bias_second = || -> i32 {
            counter += 1;
            if counter == 2 { 1000 } else { 0 }
        };
        let r1 = search(&s, &C, 1, bias_second);
        assert_ne!(r0.mv, r1.mv, "jitter should be able to swap the selected root move");
    }

    #[test]
    fn search_returns_loss_when_no_legal_moves() {
        let s = empty_state();
        let r = search(&s, &C, 4, || 0);
        assert!(
            r.score <= -MATE + 1,
            "expected losing score, got {}",
            r.score
        );
        assert_eq!(r.mv, None);
    }

    #[test]
    fn order_moves_puts_lion_capture_first() {
        let mut s = empty_state();
        place(&mut s, Sq::new(0, 1), Piece::Lion, Color::Own);
        place(&mut s, Sq::new(2, 1), Piece::Giraffe, Color::Own);
        place(&mut s, Sq::new(3, 1), Piece::Lion, Color::Opp);
        place(&mut s, Sq::new(2, 0), Piece::Chick, Color::Opp);
        let mut moves = vec![];
        s.gen_moves(&mut moves);
        order_moves(&s, &C, &mut moves);
        assert_eq!(
            moves[0],
            Move::Slide {
                from: Sq::new(2, 1),
                to: Sq::new(3, 1)
            },
            "lion capture must come first; got {:?}",
            moves
        );
    }

    #[test]
    fn order_moves_puts_higher_mvv_capture_before_lower() {
        let mut s = empty_state();
        place(&mut s, Sq::new(0, 1), Piece::Lion, Color::Own);
        place(&mut s, Sq::new(3, 0), Piece::Lion, Color::Opp);
        place(&mut s, Sq::new(2, 1), Piece::Giraffe, Color::Own);
        place(&mut s, Sq::new(3, 1), Piece::Giraffe, Color::Opp);
        place(&mut s, Sq::new(2, 0), Piece::Chick, Color::Opp);
        let mut moves = vec![];
        s.gen_moves(&mut moves);
        order_moves(&s, &C, &mut moves);
        assert_eq!(
            moves[0],
            Move::Slide {
                from: Sq::new(2, 1),
                to: Sq::new(3, 1)
            }
        );
    }

    #[test]
    fn order_moves_keeps_quiet_moves_after_captures() {
        let mut s = empty_state();
        place(&mut s, Sq::new(0, 1), Piece::Lion, Color::Own);
        place(&mut s, Sq::new(3, 1), Piece::Lion, Color::Opp);
        place(&mut s, Sq::new(2, 1), Piece::Giraffe, Color::Own);
        place(&mut s, Sq::new(0, 0), Piece::Chick, Color::Own);
        let mut moves = vec![];
        s.gen_moves(&mut moves);
        order_moves(&s, &C, &mut moves);
        let captures: Vec<_> = moves
            .iter()
            .enumerate()
            .filter(|(_, m)| is_capture(&s, **m))
            .collect();
        let quiets: Vec<_> = moves
            .iter()
            .enumerate()
            .filter(|(_, m)| !is_capture(&s, **m))
            .collect();
        for (ci, _) in &captures {
            for (qi, _) in &quiets {
                assert!(ci < qi, "capture at {ci} must come before quiet at {qi}");
            }
        }
    }

    fn is_capture(s: &State, m: Move) -> bool {
        match m {
            Move::Slide { to, .. } => s.at(to).is_some(),
            Move::Drop { .. } => false,
        }
    }
}
