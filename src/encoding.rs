use rsshogi::{
    board::Position,
    types::{Color, HandPiece, Piece, Square},
};

pub const DIM: usize = 868;
pub const CATEGORICAL_DIM: usize = 81 * 28 + 58;
pub const HAND_CAPS: [usize; 7] = [9, 4, 4, 4, 2, 2, 4];

/// Turn-relative square-major bits: own, P,L,N,S,B,R,G,K, promoted.
pub fn encode(position: &Position) -> [u8; DIM] {
    let mut bits = [0; DIM];
    for sq in Square::iter() {
        let piece = position.piece_on(sq);
        if piece == Piece::NONE {
            continue;
        }
        let normalized = if position.turn() == Color::WHITE {
            sq.flip()
        } else {
            sq
        };
        let offset = normalized.to_index() * 10;
        bits[offset] = u8::from(piece.color() == position.turn());
        bits[offset + piece.piece_type().demote().to_index()] = 1;
        bits[offset + 9] = u8::from(piece.piece_type().is_promoted());
    }
    let mut offset = 810;
    for color in [position.turn(), !position.turn()] {
        for (hp, cap) in HandPiece::iter().zip(HAND_CAPS) {
            let count = (position.hand(color).count(hp) as usize).min(cap);
            bits[offset..offset + count].fill(1);
            offset += cap;
        }
    }
    bits
}

/// A fixed conjunction of square, ownership, base type and promotion; no learned encoder.
pub fn encode_categorical(position: &Position) -> [u8; CATEGORICAL_DIM] {
    let mut bits = [0; CATEGORICAL_DIM];
    for sq in Square::iter() {
        let piece = position.piece_on(sq);
        if piece == Piece::NONE {
            continue;
        }
        let normalized = if position.turn() == Color::WHITE {
            sq.flip()
        } else {
            sq
        };
        let kind = piece.piece_type();
        let category = kind.demote().to_index() - 1 + if kind.is_promoted() { 8 } else { 0 };
        let side = usize::from(piece.color() != position.turn());
        bits[normalized.to_index() * 28 + side * 14 + category] = 1;
    }
    bits[81 * 28..].copy_from_slice(&encode(position)[810..]);
    bits
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsshogi::{board, labels::policy::CompactMoveLabel};
    #[test]
    fn initial_and_flipped_match() {
        let p = board::hirate_position();
        let bits = encode(&p);
        assert_eq!(bits.iter().map(|&b| b as usize).sum::<usize>(), 60);
        let categorical = encode_categorical(&p);
        assert_eq!(categorical.iter().map(|&b| b as usize).sum::<usize>(), 40);
        assert_eq!(
            categorical,
            encode_categorical(&board::position_from_sfen(&p.to_sfen_flipped(None)).unwrap())
        );
        assert_eq!(
            bits,
            encode(&board::position_from_sfen(&p.to_sfen_flipped(None)).unwrap())
        );
        let mut moves = board::Move32List::new();
        board::generate_legal_all_move32(&p, &mut moves);
        let mut labels: Vec<_> = moves
            .iter()
            .map(|&m| CompactMoveLabel::from_move32(m, p.turn()).unwrap().raw())
            .collect();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), 30);
        assert!(labels.iter().all(|&l| l < 1496));
    }
    #[test]
    fn every_piece_and_side_is_distinct_and_normalizes() {
        let mut seen = std::collections::HashSet::new();
        let mut categorical_seen = std::collections::HashSet::new();
        for symbol in [
            "P", "L", "N", "S", "B", "R", "G", "+P", "+L", "+N", "+S", "+B", "+R",
        ] {
            for s in [symbol.to_owned(), symbol.to_lowercase()] {
                let p =
                    board::position_from_sfen(&format!("4k4/9/9/9/4{s}4/9/9/9/4K4 b - 1")).unwrap();
                let bits = encode(&p);
                assert!(seen.insert(bits));
                let categorical = encode_categorical(&p);
                assert!(categorical_seen.insert(categorical));
                assert_eq!(
                    categorical,
                    encode_categorical(
                        &board::position_from_sfen(&p.to_sfen_flipped(None)).unwrap()
                    )
                );
                assert_eq!(
                    bits,
                    encode(&board::position_from_sfen(&p.to_sfen_flipped(None)).unwrap())
                );
            }
        }
    }
    #[test]
    fn pawn_cap_changes_input_only() {
        let make =
            |n| board::position_from_sfen(&format!("4k4/9/9/9/9/9/9/9/4K4 b {n}P 1")).unwrap();
        assert_ne!(encode(&make(8)), encode(&make(9)));
        assert_eq!(encode(&make(9)), encode(&make(18)));
        assert_eq!(make(18).hand(Color::BLACK).count(HandPiece::PAWN), 18);
        let p = board::position_from_sfen("4k4/9/9/9/9/9/9/9/4K4 b 2R2B4G4S4N4L9P 1").unwrap();
        assert_eq!(encode(&p)[810..839], [1; 29]);
        assert_eq!(encode(&p)[839..], [0; 29]);
    }
}
