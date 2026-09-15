use fly_shogi_lab::{
    encoding,
    lif::{Graph, Simulation, simulate},
};
use rsshogi::{
    board,
    labels::policy::CompactMoveLabel,
    types::{Color, HandPiece},
};

#[test]
fn captures_promotion_and_drop_preserve_rule_state() {
    let mut p = board::hirate_position();
    for usi in ["7g7f", "3c3d", "8h2b+", "3a2b", "B*4e"] {
        let mut legal = board::Move32List::new();
        board::generate_legal_all_move32(&p, &mut legal);
        let mut labels = std::collections::HashSet::new();
        for &m in legal.iter() {
            let l = CompactMoveLabel::from_move32(m, p.turn()).unwrap().raw();
            assert!(l < 1496 && labels.insert(l));
        }
        let mv = *legal
            .iter()
            .find(|m| m.to_string() == usi)
            .expect("legal fixture move");
        let before = encoding::encode(&p);
        p.apply_move32(mv);
        assert_ne!(before, encoding::encode(&p));
        assert_eq!(
            encoding::encode(&p),
            encoding::encode(&board::position_from_sfen(&p.to_sfen_flipped(None)).unwrap())
        );
    }
    assert_eq!(p.hand(Color::BLACK).count(HandPiece::BISHOP), 0);
    assert_eq!(p.hand(Color::WHITE).count(HandPiece::BISHOP), 1);
    // White is now to move; bishop hand segment starts after P,L,N,S.
    assert_eq!(&encoding::encode(&p)[831..833], &[1, 0]);
}

#[test]
fn no_drive_has_no_hidden_activity() {
    let graph = Graph {
        ids: vec![10, 20],
        offsets: vec![0, 1, 2],
        targets: vec![1, 0],
        weights: vec![1000, 1000],
    };
    let cfg = Simulation {
        steps: 1200,
        direct: vec![0],
        active: vec![false],
        rate_hz: 150.0,
        seed: 101,
        outputs: vec![1],
        disconnected: false,
        events: None,
        trace: false,
    };
    let result = simulate(&graph, &cfg).unwrap();
    assert_eq!(result.counts, [0, 0]);
    assert_eq!(result.output_z, [0.0]);
    assert_eq!(result.stimulus_events, 0);
}
