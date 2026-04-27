//! Rules of Dōbutsu shōgi.
//!
//! Board: 3 columns × 4 rows = 12 squares. Square index = row * 3 + col.
//! State is stored side-relative: pieces are labelled "Own" / "Opp" from
//! the perspective of the side to move. `apply` ends with a 180° rotation
//! plus color swap so the new mover sees their pieces as "Own".
//!
//! Rows from "Own" perspective:
//!   row 0 = Own back rank
//!   row 3 = Opp back rank (the Try goal for Own's Lion)
//!
//! Try rule (eager): if after applying a move the mover's Lion sits on the
//! opponent's back rank and is not attacked there, the mover wins by Try.

use std::fmt;

pub const COLS: i8 = 3;
pub const ROWS: i8 = 4;
pub const N_SQUARES: usize = 12;

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub enum Piece {
    Lion,
    Giraffe,
    Elephant,
    Chick,
    Hen,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub enum Color {
    Own,
    Opp,
}

impl Color {
    fn flip(self) -> Color {
        match self {
            Color::Own => Color::Opp,
            Color::Opp => Color::Own,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct Stone {
    pub piece: Piece,
    pub color: Color,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct Sq(pub u8);

impl Sq {
    pub fn new(row: i8, col: i8) -> Self {
        debug_assert!((0..ROWS).contains(&row) && (0..COLS).contains(&col));
        Sq((row * COLS + col) as u8)
    }
    pub fn row(self) -> i8 {
        (self.0 as i8) / COLS
    }
    pub fn col(self) -> i8 {
        (self.0 as i8) % COLS
    }
    pub fn from_rc(row: i8, col: i8) -> Option<Self> {
        if (0..ROWS).contains(&row) && (0..COLS).contains(&col) {
            Some(Sq::new(row, col))
        } else {
            None
        }
    }
    pub fn rotate180(self) -> Sq {
        Sq::new(ROWS - 1 - self.row(), COLS - 1 - self.col())
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub enum Move {
    Slide {
        from: Sq,
        to: Sq,
    },
    /// `piece` must be Chick, Elephant, or Giraffe (Lion is never in hand;
    /// captured Hens revert to Chick).
    Drop {
        piece: Piece,
        to: Sq,
    },
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash, Default)]
pub struct Hand {
    pub chick: u8,
    pub elephant: u8,
    pub giraffe: u8,
}

impl Hand {
    fn add(&mut self, captured: Piece) {
        match captured {
            // promoted Hens revert to Chick when captured
            Piece::Chick | Piece::Hen => self.chick += 1,
            Piece::Elephant => self.elephant += 1,
            Piece::Giraffe => self.giraffe += 1,
            Piece::Lion => unreachable!("lion capture ends the game; not added to hand"),
        }
    }
    fn take(&mut self, p: Piece) {
        match p {
            Piece::Chick => self.chick -= 1,
            Piece::Elephant => self.elephant -= 1,
            Piece::Giraffe => self.giraffe -= 1,
            _ => unreachable!("only Chick/Elephant/Giraffe live in hand"),
        }
    }
    pub fn count(&self, p: Piece) -> u8 {
        match p {
            Piece::Chick => self.chick,
            Piece::Elephant => self.elephant,
            Piece::Giraffe => self.giraffe,
            _ => 0,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct State {
    pub board: [Option<Stone>; N_SQUARES],
    pub own_hand: Hand,
    pub opp_hand: Hand,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Outcome {
    Ongoing,
    /// The side that just moved wins by capturing the opposing Lion.
    LionCaptured,
    /// The side that just moved wins by Try.
    Try,
}

impl Outcome {
    pub fn is_terminal(self) -> bool {
        self != Outcome::Ongoing
    }
}

impl State {
    pub fn initial() -> Self {
        let own = |p| {
            Some(Stone {
                piece: p,
                color: Color::Own,
            })
        };
        let opp = |p| {
            Some(Stone {
                piece: p,
                color: Color::Opp,
            })
        };
        let mut board = [None; N_SQUARES];
        // Own back rank (row 0): E L G   (col 0..2)
        board[Sq::new(0, 0).0 as usize] = own(Piece::Elephant);
        board[Sq::new(0, 1).0 as usize] = own(Piece::Lion);
        board[Sq::new(0, 2).0 as usize] = own(Piece::Giraffe);
        // Own Chick advances forward (row 1, col 1)
        board[Sq::new(1, 1).0 as usize] = own(Piece::Chick);
        // Opp Chick mirrored
        board[Sq::new(2, 1).0 as usize] = opp(Piece::Chick);
        // Opp back rank (row 3): G L E   (180°-mirror of Own's back rank)
        board[Sq::new(3, 0).0 as usize] = opp(Piece::Giraffe);
        board[Sq::new(3, 1).0 as usize] = opp(Piece::Lion);
        board[Sq::new(3, 2).0 as usize] = opp(Piece::Elephant);
        State {
            board,
            own_hand: Hand::default(),
            opp_hand: Hand::default(),
        }
    }

    pub fn at(&self, sq: Sq) -> Option<Stone> {
        self.board[sq.0 as usize]
    }

    /// Outcome derivable from the position alone, before the side to move plays.
    ///
    /// Returns `Outcome::Try` when Own's Lion is on row 3 — i.e., the previous
    /// mover placed it there in check, the opponent failed to capture it, and
    /// the side to move now wins by Try (Lion survived a full turn on the
    /// opponent's back rank).
    ///
    /// Never returns `LionCaptured`: a Lion capture is reported by `apply`
    /// at the moment it happens, and a state with Own's Lion missing isn't
    /// reachable from legal play.
    ///
    /// Search-loop pattern:
    /// ```ignore
    /// match state.immediate_outcome() {
    ///     Outcome::Ongoing => { /* gen_moves, recurse on apply results */ }
    ///     terminal => return /* score for STM */,
    /// }
    /// ```
    pub fn immediate_outcome(&self) -> Outcome {
        for i in 0..N_SQUARES as u8 {
            if matches!(
                self.board[i as usize],
                Some(Stone {
                    piece: Piece::Lion,
                    color: Color::Own
                })
            ) {
                if Sq(i).row() == ROWS - 1 {
                    return Outcome::Try;
                }
                break;
            }
        }
        Outcome::Ongoing
    }

    /// All pseudo-legal moves for the side to move ("Own").
    /// The mover may move into a square attacked by the opponent — the
    /// resulting position simply lets the opponent capture the Lion next.
    /// Drops are allowed on any empty square (no DS-specific restrictions).
    pub fn gen_moves(&self, out: &mut Vec<Move>) {
        out.clear();
        for s in 0..N_SQUARES as u8 {
            let sq = Sq(s);
            let Some(stone) = self.at(sq) else { continue };
            if stone.color != Color::Own {
                continue;
            }
            for &(dx, dy) in piece_offsets(stone.piece, Color::Own) {
                let Some(to) = Sq::from_rc(sq.row() + dy, sq.col() + dx) else {
                    continue;
                };
                if let Some(t) = self.at(to) {
                    if t.color == Color::Own {
                        continue;
                    }
                }
                out.push(Move::Slide { from: sq, to });
            }
        }
        for &p in &[Piece::Chick, Piece::Elephant, Piece::Giraffe] {
            if self.own_hand.count(p) == 0 {
                continue;
            }
            for s in 0..N_SQUARES as u8 {
                let sq = Sq(s);
                if self.at(sq).is_none() {
                    out.push(Move::Drop { piece: p, to: sq });
                }
            }
        }
    }

    /// Apply a move. Returns `(next_state, outcome)`.
    /// On non-terminal outcomes, `next_state` is flipped (opponent now
    /// sees their pieces as Own). On terminal outcomes the state is
    /// returned unflipped, so the mover's side is still "Own".
    pub fn apply(&self, m: Move) -> (State, Outcome) {
        let mut s = *self;
        match m {
            Move::Slide { from, to } => {
                let mut stone = s.board[from.0 as usize].take().expect("slide from empty");
                debug_assert_eq!(stone.color, Color::Own);
                if let Some(target) = s.board[to.0 as usize] {
                    debug_assert_eq!(target.color, Color::Opp);
                    if target.piece == Piece::Lion {
                        s.board[to.0 as usize] = Some(stone);
                        return (s, Outcome::LionCaptured);
                    }
                    s.own_hand.add(target.piece);
                }
                if stone.piece == Piece::Chick && to.row() == ROWS - 1 {
                    stone.piece = Piece::Hen;
                }
                s.board[to.0 as usize] = Some(stone);
            }
            Move::Drop { piece, to } => {
                debug_assert!(matches!(
                    piece,
                    Piece::Chick | Piece::Elephant | Piece::Giraffe
                ));
                debug_assert!(s.board[to.0 as usize].is_none());
                s.own_hand.take(piece);
                s.board[to.0 as usize] = Some(Stone {
                    piece,
                    color: Color::Own,
                });
            }
        }
        if own_lion_on_try_rank_unattacked(&s) {
            return (s, Outcome::Try);
        }
        (s.flip(), Outcome::Ongoing)
    }

    /// 180° rotate the board, swap piece colors, swap hands.
    pub fn flip(&self) -> State {
        let mut new_board = [None; N_SQUARES];
        for s in 0..N_SQUARES as u8 {
            let sq = Sq(s);
            if let Some(stone) = self.at(sq) {
                new_board[sq.rotate180().0 as usize] = Some(Stone {
                    piece: stone.piece,
                    color: stone.color.flip(),
                });
            }
        }
        State {
            board: new_board,
            own_hand: self.opp_hand,
            opp_hand: self.own_hand,
        }
    }
}

fn own_lion_on_try_rank_unattacked(s: &State) -> bool {
    let lion_sq = (0..N_SQUARES as u8).find(|&i| {
        matches!(
            s.board[i as usize],
            Some(Stone {
                piece: Piece::Lion,
                color: Color::Own
            })
        )
    });
    let Some(ls) = lion_sq else { return false };
    let ls = Sq(ls);
    if ls.row() != ROWS - 1 {
        return false;
    }
    !is_attacked_by_opp(s, ls)
}

fn is_attacked_by_opp(s: &State, target: Sq) -> bool {
    for i in 0..N_SQUARES as u8 {
        let sq = Sq(i);
        let Some(stone) = s.at(sq) else { continue };
        if stone.color != Color::Opp {
            continue;
        }
        for &(dx, dy) in piece_offsets(stone.piece, Color::Opp) {
            if let Some(to) = Sq::from_rc(sq.row() + dy, sq.col() + dx) {
                if to == target {
                    return true;
                }
            }
        }
    }
    false
}

/// Movement offsets `(dx, dy)` for a piece of given color.
/// "Forward" for Own = +y (toward higher row); for Opp = -y.
fn piece_offsets(p: Piece, c: Color) -> &'static [(i8, i8)] {
    match (p, c) {
        (Piece::Lion, _) => &[
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ],
        (Piece::Giraffe, _) => &[(0, -1), (-1, 0), (1, 0), (0, 1)],
        (Piece::Elephant, _) => &[(-1, -1), (1, -1), (-1, 1), (1, 1)],
        (Piece::Chick, Color::Own) => &[(0, 1)],
        (Piece::Chick, Color::Opp) => &[(0, -1)],
        // Gold-general: 8 king dirs minus the two backward diagonals.
        (Piece::Hen, Color::Own) => &[(-1, 1), (0, 1), (1, 1), (-1, 0), (1, 0), (0, -1)],
        (Piece::Hen, Color::Opp) => &[(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (0, 1)],
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "opp hand: C={} E={} G={}",
            self.opp_hand.chick, self.opp_hand.elephant, self.opp_hand.giraffe
        )?;
        for r in (0..ROWS).rev() {
            for c in 0..COLS {
                let ch = match self.at(Sq::new(r, c)) {
                    None => '.',
                    Some(Stone { piece, color }) => {
                        let ch = match piece {
                            Piece::Lion => 'L',
                            Piece::Giraffe => 'G',
                            Piece::Elephant => 'E',
                            Piece::Chick => 'C',
                            Piece::Hen => 'H',
                        };
                        if color == Color::Own {
                            ch
                        } else {
                            ch.to_ascii_lowercase()
                        }
                    }
                };
                write!(f, "{} ", ch)?;
            }
            writeln!(f)?;
        }
        writeln!(
            f,
            "own hand: C={} E={} G={}",
            self.own_hand.chick, self.own_hand.elephant, self.own_hand.giraffe
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_position_has_four_legal_moves() {
        // Lion(0,1): targets (1,0) and (1,2)               -> 2
        // Elephant(0,0): all diagonal targets blocked/oob  -> 0
        // Giraffe(0,2): only (1,2) reachable               -> 1
        // Chick(1,1): forward to (2,1), captures opp chick -> 1
        // Total: 4
        let s = State::initial();
        let mut moves = vec![];
        s.gen_moves(&mut moves);
        assert_eq!(moves.len(), 4, "moves: {:?}", moves);
    }

    #[test]
    fn chick_capture_goes_to_hand_and_flips_state() {
        let s = State::initial();
        let m = Move::Slide {
            from: Sq::new(1, 1),
            to: Sq::new(2, 1),
        };
        let (s2, out) = s.apply(m);
        assert_eq!(out, Outcome::Ongoing);
        // After flip: the captured chick now belongs to the *previous* mover,
        // which from new-STM POV is "opp".
        assert_eq!(s2.opp_hand.chick, 1);
        assert_eq!(s2.own_hand.chick, 0);
    }

    #[test]
    fn flip_is_involution() {
        let s = State::initial();
        assert_eq!(s.flip().flip(), s);
    }

    #[test]
    fn flipping_initial_state_is_initial_state() {
        // Initial position is symmetric under 180° + color swap.
        let s = State::initial();
        assert_eq!(s.flip(), s);
    }

    #[test]
    fn lion_capture_is_terminal_and_unflipped() {
        // Place own Lion next to opp Lion, capture.
        let mut board = [None; N_SQUARES];
        board[Sq::new(0, 1).0 as usize] = Some(Stone {
            piece: Piece::Lion,
            color: Color::Own,
        });
        board[Sq::new(1, 1).0 as usize] = Some(Stone {
            piece: Piece::Lion,
            color: Color::Opp,
        });
        let s = State {
            board,
            own_hand: Hand::default(),
            opp_hand: Hand::default(),
        };
        let (s2, out) = s.apply(Move::Slide {
            from: Sq::new(0, 1),
            to: Sq::new(1, 1),
        });
        assert_eq!(out, Outcome::LionCaptured);
        // unflipped: own lion still on board as Own
        assert_eq!(
            s2.at(Sq::new(1, 1)),
            Some(Stone {
                piece: Piece::Lion,
                color: Color::Own
            })
        );
    }

    #[test]
    fn try_win_when_lion_reaches_back_rank_unattacked() {
        // Own Lion at (2,0), nothing attacking (3,0). Move Lion to (3,0) = Try.
        let mut board = [None; N_SQUARES];
        board[Sq::new(2, 0).0 as usize] = Some(Stone {
            piece: Piece::Lion,
            color: Color::Own,
        });
        // Park opp Lion in a corner where it can't attack (3,0).
        board[Sq::new(0, 2).0 as usize] = Some(Stone {
            piece: Piece::Lion,
            color: Color::Opp,
        });
        let s = State {
            board,
            own_hand: Hand::default(),
            opp_hand: Hand::default(),
        };
        let (_s2, out) = s.apply(Move::Slide {
            from: Sq::new(2, 0),
            to: Sq::new(3, 0),
        });
        assert_eq!(out, Outcome::Try);
    }

    #[test]
    fn no_try_when_lion_on_back_rank_is_attacked() {
        // Own Lion to (3,0); opp Giraffe at (3,1) attacks (3,0). Not a Try.
        let mut board = [None; N_SQUARES];
        board[Sq::new(2, 0).0 as usize] = Some(Stone {
            piece: Piece::Lion,
            color: Color::Own,
        });
        board[Sq::new(3, 1).0 as usize] = Some(Stone {
            piece: Piece::Giraffe,
            color: Color::Opp,
        });
        board[Sq::new(0, 2).0 as usize] = Some(Stone {
            piece: Piece::Lion,
            color: Color::Opp,
        });
        let s = State {
            board,
            own_hand: Hand::default(),
            opp_hand: Hand::default(),
        };
        let (_s2, out) = s.apply(Move::Slide {
            from: Sq::new(2, 0),
            to: Sq::new(3, 0),
        });
        assert_eq!(out, Outcome::Ongoing);
    }

    #[test]
    fn chick_promotes_on_reaching_last_rank() {
        let mut board = [None; N_SQUARES];
        board[Sq::new(2, 0).0 as usize] = Some(Stone {
            piece: Piece::Chick,
            color: Color::Own,
        });
        // Lions on board to keep state legal-ish; not strictly required.
        board[Sq::new(0, 1).0 as usize] = Some(Stone {
            piece: Piece::Lion,
            color: Color::Own,
        });
        board[Sq::new(3, 1).0 as usize] = Some(Stone {
            piece: Piece::Lion,
            color: Color::Opp,
        });
        let s = State {
            board,
            own_hand: Hand::default(),
            opp_hand: Hand::default(),
        };
        let (s2, out) = s.apply(Move::Slide {
            from: Sq::new(2, 0),
            to: Sq::new(3, 0),
        });
        assert_eq!(out, Outcome::Ongoing);
        // After flip: from new-STM POV the promoted Hen is at the rotated square,
        // and is now an Opp Hen.
        let rotated = Sq::new(3, 0).rotate180();
        assert_eq!(
            s2.at(rotated),
            Some(Stone {
                piece: Piece::Hen,
                color: Color::Opp
            })
        );
    }

    #[test]
    fn captured_hen_returns_to_hand_as_chick() {
        // Own Hen at (1,1); opp piece at (2,1) — Hen moves forward and... wait,
        // we want the *opp* to capture our Hen. Easier: build a position where
        // it's the mover's turn to capture an opp Hen.
        let mut board = [None; N_SQUARES];
        board[Sq::new(0, 0).0 as usize] = Some(Stone {
            piece: Piece::Giraffe,
            color: Color::Own,
        });
        board[Sq::new(1, 0).0 as usize] = Some(Stone {
            piece: Piece::Hen,
            color: Color::Opp,
        });
        // Lions to satisfy debug-assertions on captures (target color).
        board[Sq::new(0, 1).0 as usize] = Some(Stone {
            piece: Piece::Lion,
            color: Color::Own,
        });
        board[Sq::new(3, 1).0 as usize] = Some(Stone {
            piece: Piece::Lion,
            color: Color::Opp,
        });
        let s = State {
            board,
            own_hand: Hand::default(),
            opp_hand: Hand::default(),
        };
        let (s2, out) = s.apply(Move::Slide {
            from: Sq::new(0, 0),
            to: Sq::new(1, 0),
        });
        assert_eq!(out, Outcome::Ongoing);
        // After flip the captured-Hen-as-Chick is in opp_hand from new STM POV.
        assert_eq!(s2.opp_hand.chick, 1);
    }

    fn perft(s: &State, depth: u32) -> u64 {
        if s.immediate_outcome().is_terminal() {
            return 1;
        }
        if depth == 0 {
            return 1;
        }
        let mut moves = vec![];
        s.gen_moves(&mut moves);
        let mut total = 0;
        for m in moves {
            let (s2, out) = s.apply(m);
            total += if out.is_terminal() {
                1
            } else {
                perft(&s2, depth - 1)
            };
        }
        total
    }

    #[test]
    fn pre_move_try_when_lion_already_on_back_rank() {
        // Construct a state where Own's Lion is on row 3 (suboptimal previous play
        // by the opponent: they failed to capture the lion they had attacked).
        let mut board = [None; N_SQUARES];
        board[Sq::new(3, 0).0 as usize] = Some(Stone {
            piece: Piece::Lion,
            color: Color::Own,
        });
        board[Sq::new(0, 2).0 as usize] = Some(Stone {
            piece: Piece::Lion,
            color: Color::Opp,
        });
        let s = State {
            board,
            own_hand: Hand::default(),
            opp_hand: Hand::default(),
        };
        assert_eq!(s.immediate_outcome(), Outcome::Try);
    }

    #[test]
    fn pre_move_no_try_in_initial_position() {
        assert_eq!(State::initial().immediate_outcome(), Outcome::Ongoing);
    }

    /// Regression for the rule-tightening: under the previous (eager-only)
    /// implementation the search would never detect this win, because the
    /// eager check only fires inside `apply`, and from this state the
    /// winning side has not moved yet.
    ///
    /// Scenario, as a single ply: it's B's turn (B = "Own" in this state).
    /// B has a Giraffe at (1,1) that could capture A's Lion sitting on B's
    /// back rank (0,1). B chooses *not* to capture — a suboptimal but legal
    /// move. After B's move and the flip, A's Lion is on A's row 3 again,
    /// and A wins by Try (Lion survived a full turn on the back rank).
    /// `immediate_outcome` reports this without A needing to play.
    #[test]
    fn try_wins_at_start_of_turn_when_opponent_skipped_capture() {
        let mut board = [None; N_SQUARES];
        // A's Lion (Opp here) sits on B's back rank.
        board[Sq::new(0, 1).0 as usize] = Some(Stone {
            piece: Piece::Lion,
            color: Color::Opp,
        });
        // B's own Lion, well clear of the action.
        board[Sq::new(2, 1).0 as usize] = Some(Stone {
            piece: Piece::Lion,
            color: Color::Own,
        });
        // B's Giraffe at (1,1) — can capture (0,1) by moving forward.
        board[Sq::new(1, 1).0 as usize] = Some(Stone {
            piece: Piece::Giraffe,
            color: Color::Own,
        });
        let s = State {
            board,
            own_hand: Hand::default(),
            opp_hand: Hand::default(),
        };
        // Sanity: the capture *is* available as a legal move.
        let mut moves = vec![];
        s.gen_moves(&mut moves);
        assert!(moves.contains(&Move::Slide {
            from: Sq::new(1, 1),
            to: Sq::new(0, 1),
        }));

        // B picks a non-capture: Giraffe sideways instead.
        let (next, out) = s.apply(Move::Slide {
            from: Sq::new(1, 1),
            to: Sq::new(1, 0),
        });
        // Eager check inside `apply` finds nothing — B's own Lion is not on row 3.
        assert_eq!(out, Outcome::Ongoing);

        // The *only* signal that A has won is the start-of-turn check.
        assert_eq!(next.immediate_outcome(), Outcome::Try);
    }

    #[test]
    fn perft_depth_1_is_4() {
        assert_eq!(perft(&State::initial(), 1), 4);
    }

    #[test]
    fn perft_depth_2_runs() {
        // No published reference value handy; just sanity-check it terminates
        // and grows. We'll calibrate against a reference once we have one.
        let n = perft(&State::initial(), 2);
        assert!(n > 4, "perft(2) = {}", n);
    }
}
