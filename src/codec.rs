//! Cross-boundary move encoding (Rust ⇄ TypeScript).
//!
//! `code = (from << 4) | to` where:
//!   - `to`   in 0..=11 (square index)
//!   - `from` in 0..=11 for a slide, or 12/13/14 for a Chick/Elephant/Giraffe drop.
//!
//! Code 15 in `from` is unused.

use crate::rules::{Move, Piece, Sq};

pub const DROP_CHICK: u8 = 12;
pub const DROP_ELEPHANT: u8 = 13;
pub const DROP_GIRAFFE: u8 = 14;

pub fn encode(m: Move) -> u8 {
    let (from_n, to) = match m {
        Move::Slide { from, to } => (from.0, to),
        Move::Drop { piece, to } => {
            let from = match piece {
                Piece::Chick => DROP_CHICK,
                Piece::Elephant => DROP_ELEPHANT,
                Piece::Giraffe => DROP_GIRAFFE,
                Piece::Lion | Piece::Hen => panic!("piece never in hand: {piece:?}"),
            };
            (from, to)
        }
    };
    (from_n << 4) | (to.0 & 0x0F)
}

pub fn decode(code: u8) -> Option<Move> {
    let to_n = code & 0x0F;
    let from_n = code >> 4;
    if to_n >= 12 {
        return None;
    }
    let to = Sq(to_n);
    if from_n < 12 {
        return Some(Move::Slide { from: Sq(from_n), to });
    }
    let piece = match from_n {
        DROP_CHICK => Piece::Chick,
        DROP_ELEPHANT => Piece::Elephant,
        DROP_GIRAFFE => Piece::Giraffe,
        _ => return None,
    };
    Some(Move::Drop { piece, to })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Sq;

    #[test]
    fn slide_round_trip() {
        let m = Move::Slide {
            from: Sq::new(2, 1),
            to: Sq::new(3, 1),
        };
        let code = encode(m);
        // from = 2*3+1 = 7, to = 3*3+1 = 10. code = (7<<4)|10 = 0x7A = 122.
        assert_eq!(code, 0x7A);
        assert_eq!(decode(code), Some(m));
    }

    #[test]
    fn drop_chick_round_trip() {
        let m = Move::Drop {
            piece: Piece::Chick,
            to: Sq::new(1, 1),
        };
        let code = encode(m);
        // from-field = 12, to = 4. code = (12<<4)|4 = 0xC4.
        assert_eq!(code, 0xC4);
        assert_eq!(decode(code), Some(m));
    }

    #[test]
    fn drop_elephant_round_trip() {
        let m = Move::Drop {
            piece: Piece::Elephant,
            to: Sq::new(0, 0),
        };
        assert_eq!(decode(encode(m)), Some(m));
    }

    #[test]
    fn drop_giraffe_round_trip() {
        let m = Move::Drop {
            piece: Piece::Giraffe,
            to: Sq::new(3, 2),
        };
        assert_eq!(decode(encode(m)), Some(m));
    }

    #[test]
    fn decode_rejects_out_of_range_to() {
        // to-nibble 12..=15 are invalid (only 0..=11 are squares).
        for to in 12u8..=15 {
            assert_eq!(decode(to), None, "to={to} should be invalid");
        }
    }

    #[test]
    fn decode_rejects_reserved_from_15() {
        let code = 15u8 << 4;
        assert_eq!(decode(code), None);
    }

    #[test]
    fn all_legal_codes_round_trip() {
        // Every slide (from 0..=11, to 0..=11) and every drop (12..=14, to 0..=11).
        for from in 0u8..=11 {
            for to in 0u8..=11 {
                let m = Move::Slide {
                    from: Sq(from),
                    to: Sq(to),
                };
                assert_eq!(decode(encode(m)), Some(m));
            }
        }
        for (drop_from, piece) in [
            (DROP_CHICK, Piece::Chick),
            (DROP_ELEPHANT, Piece::Elephant),
            (DROP_GIRAFFE, Piece::Giraffe),
        ] {
            for to in 0u8..=11 {
                let m = Move::Drop {
                    piece,
                    to: Sq(to),
                };
                let code = encode(m);
                assert_eq!(code >> 4, drop_from);
                assert_eq!(decode(code), Some(m));
            }
        }
    }
}
