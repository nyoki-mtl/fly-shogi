//! Keep legality and both label formats authoritative in rsshogi.
use rsshogi::{board, labels::policy::CompactMoveLabel};
use serde_json::{Value, json};
use std::{
    error::Error,
    fs,
    io::{BufRead, BufReader, BufWriter, Write},
};
fn main() -> Result<(), Box<dyn Error>> {
    let a: Vec<_> = std::env::args().collect();
    if a.len() != 3 {
        return Err("Usage: export-policy-labels INPUT_JSONL OUTPUT_JSONL".into());
    }
    let mut out = BufWriter::new(
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&a[2])?,
    );
    for line in BufReader::new(fs::File::open(&a[1])?).lines() {
        let mut r: Value = serde_json::from_str(&line?)?;
        let p = board::position_from_sfen(r["sfen"].as_str().ok_or("Missing SFEN")?)?;
        let mut moves = board::Move32List::new();
        board::generate_legal_all_move32(&p, &mut moves);
        r["legal_labels"] = json!(
            moves
                .iter()
                .map(|&m| {
                    let compact = CompactMoveLabel::from_move32(m, p.turn()).unwrap();
                    (compact.raw(), compact.expand().raw(), m.to_string())
                })
                .collect::<Vec<_>>()
        );
        writeln!(out, "{r}")?;
    }
    out.flush()?;
    Ok(())
}
