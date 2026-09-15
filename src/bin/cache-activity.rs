//! Record full-circuit presynaptic activity for bounded synapse optimization.
use fly_shogi_lab::{lif::Graph, plasticity::Circuit};
use rsshogi::{board, labels::policy::CompactMoveLabel};
use serde::Deserialize;
use serde_json::json;
use std::{
    error::Error,
    fs,
    io::{BufWriter, Write},
    path::Path,
};

#[derive(Deserialize)]
struct Pools {
    input_pool: Vec<usize>,
}
#[derive(Deserialize)]
struct Teacher {
    sfen: String,
    seed: u64,
    target: usize,
    split: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let a: Vec<_> = std::env::args().collect();
    if a.len() != 6 {
        return Err("Usage: cache-activity GRAPH METADATA MODEL TEACHERS OUTPUT_DIR".into());
    }
    let graph = Graph::read(Path::new(&a[1]))?;
    let pools: Pools = serde_json::from_slice(&fs::read(&a[2])?)?;
    let model: Circuit = serde_json::from_slice(&fs::read(&a[3])?)?;
    model.validate(&graph)?;
    let teachers: Vec<Teacher> = fs::read_to_string(&a[4])?
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;
    if !teachers.iter().any(|r| r.split == "train")
        || !teachers.iter().any(|r| r.split == "validation")
    {
        return Err("Both train and validation positions are required".into());
    }
    let output = Path::new(&a[5]);
    fs::create_dir(output)?;
    fs::copy(&a[3], output.join("initial.json"))?;
    fs::copy(&a[4], output.join("teachers.jsonl"))?;
    let partial = output.join("spikes.partial.bin");
    let mut stream = BufWriter::new(fs::File::create(&partial)?);
    let dense = model.dense(&graph);
    let mut completed = 0;
    for chunk in teachers.chunks(8) {
        let batch = std::thread::scope(|scope| {
            let handles: Vec<_> = chunk
                .iter()
                .map(|row| {
                    let (graph, model, pools, dense) = (&graph, &model, &pools, &dense);
                    scope.spawn(move || -> Result<Vec<u8>, String> {
                        if !["train", "validation"].contains(&row.split.as_str()) {
                            return Err("Invalid split".into());
                        }
                        let position =
                            board::position_from_sfen(&row.sfen).map_err(|e| e.to_string())?;
                        let mut legal = board::Move32List::new();
                        board::generate_legal_all_move32(&position, &mut legal);
                        if !legal.iter().any(|&m| {
                            CompactMoveLabel::from_move32(m, position.turn())
                                .unwrap()
                                .raw() as usize
                                == row.target
                        }) {
                            return Err("Illegal teacher".into());
                        }
                        let response =
                            model.run(graph, &pools.input_pool, &position, row.seed, dense)?;
                        let mut counts =
                            Vec::with_capacity(model.time_bins * model.source_neurons().len());
                        for bin in 0..model.time_bins {
                            for &source in model.source_neurons() {
                                counts.push(
                                    u8::try_from(
                                        response.policy_counts()[bin * graph.ids.len() + source],
                                    )
                                    .map_err(|_| "Spike count overflow")?,
                                );
                            }
                        }
                        Ok(counts)
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().map_err(|_| "Cache worker panicked".to_string())?)
                .collect::<Result<Vec<_>, String>>()
        })?;
        for counts in batch {
            stream.write_all(&counts)?;
            completed += 1;
        }
        if completed % 256 == 0 || completed == teachers.len() {
            println!(
                "{}",
                json!({"stage":"cache", "records":completed, "total":teachers.len()})
            );
        }
    }
    stream.flush()?;
    drop(stream);
    fs::rename(partial, output.join("spikes.bin"))?;
    Ok(())
}
