//! Persistent, f64 objective for bounded optimization of existing synapses.
use fly_shogi_lab::{lif::Graph, plasticity::Circuit};
use rsshogi::{board, labels::policy::CompactMoveLabel};
use serde::Deserialize;
use serde_json::json;
use std::{
    error::Error,
    fs,
    io::{self, BufRead, Write},
    path::Path,
    time::Instant,
};

#[derive(Deserialize)]
struct Teacher {
    sfen: String,
    target: usize,
    split: String,
}
#[derive(Deserialize)]
struct Request {
    gains: Vec<f64>,
    split: String,
    #[serde(default)]
    limit: usize,
    #[serde(default)]
    gradient: bool,
}
struct Row {
    labels: Vec<usize>,
    target: usize,
    train: bool,
}
struct Objective {
    spikes: Vec<u8>,
    rows: Vec<Row>,
    source_edges: Vec<Vec<usize>>,
    groups: Vec<usize>,
    weights: Vec<f64>,
    codes: Vec<Vec<f64>>,
    sources: usize,
    bins: usize,
    outputs: usize,
    workers: usize,
}
struct Totals {
    loss: f64,
    correct: usize,
    top5: usize,
    rank: usize,
    gradient: Vec<f64>,
    diagonal: Vec<f64>,
}
impl Objective {
    fn compute(&self, gains: &[f64], indices: &[usize], gradient: bool) -> Totals {
        let dim = self.outputs * self.bins;
        let width = self.sources * self.bins;
        let mut total = Totals {
            loss: 0.0,
            correct: 0,
            top5: 0,
            rank: 0,
            gradient: vec![0.0; if gradient { gains.len() } else { 0 }],
            diagonal: vec![0.0; if gradient { gains.len() } else { 0 }],
        };
        let mut activity = vec![0.0; dim];
        let mut norms = vec![0.0; dim];
        let mut signal = vec![0.0; dim];
        for &r in indices {
            let counts = &self.spikes[r * width..(r + 1) * width];
            activity.fill(0.0);
            norms.fill(0.0);
            for bin in 0..self.bins {
                let counts = &counts[bin * self.sources..(bin + 1) * self.sources];
                for (source, &count) in counts.iter().enumerate() {
                    if count == 0 {
                        continue;
                    }
                    for &i in &self.source_edges[source] {
                        let w = count as f64 * self.weights[i];
                        let j = bin * self.outputs + self.groups[i];
                        activity[j] += w * (gains[i] - 1.0);
                        norms[j] += w;
                    }
                }
            }
            for j in 0..dim {
                if norms[j] > 0.0 {
                    norms[j] = 1.0 / norms[j];
                    activity[j] = -activity[j] * norms[j];
                }
            }
            let row = &self.rows[r];
            let scores: Vec<_> = row
                .labels
                .iter()
                .map(|&l| {
                    activity
                        .iter()
                        .zip(&self.codes[l])
                        .map(|(a, b)| a * b)
                        .sum::<f64>()
                })
                .collect();
            let max = scores.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let mut probs: Vec<_> = scores.iter().map(|s| (s - max).exp()).collect();
            let sum: f64 = probs.iter().sum();
            total.loss += max + sum.ln() - scores[row.target];
            let target_label = row.labels[row.target];
            let rank = 1 + scores
                .iter()
                .enumerate()
                .filter(|&(i, &s)| {
                    s > scores[row.target]
                        || (s == scores[row.target] && row.labels[i] < target_label)
                })
                .count();
            total.rank += rank;
            total.correct += usize::from(rank == 1);
            total.top5 += usize::from(rank <= 5);
            if !gradient {
                continue;
            }
            signal.fill(0.0);
            for (slot, &l) in row.labels.iter().enumerate() {
                probs[slot] /= sum;
                let error = probs[slot] - f64::from(slot == row.target);
                for (s, &code) in signal.iter_mut().zip(&self.codes[l]) {
                    *s += error * code;
                }
            }
            for bin in 0..self.bins {
                let counts = &counts[bin * self.sources..(bin + 1) * self.sources];
                for (source, &count) in counts.iter().enumerate() {
                    if count == 0 {
                        continue;
                    }
                    for &i in &self.source_edges[source] {
                        let j = bin * self.outputs + self.groups[i];
                        let derivative = count as f64 * self.weights[i] * norms[j];
                        total.gradient[i] -= derivative * signal[j];
                        // A coordinate scale, not an added model parameter or exact Hessian.
                        total.diagonal[i] += derivative * derivative * self.codes[0][j].powi(2);
                    }
                }
            }
        }
        total
    }
    fn evaluate(&self, request: &Request) -> serde_json::Value {
        let start = Instant::now();
        let mut indices: Vec<_> = self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, r)| r.train == (request.split == "train"))
            .map(|(i, _)| i)
            .collect();
        if request.limit > 0 {
            indices.truncate(request.limit);
        }
        let partials = std::thread::scope(|scope| {
            let handles: Vec<_> = indices
                .chunks(indices.len().div_ceil(self.workers))
                .map(|part| {
                    scope.spawn(move || self.compute(&request.gains, part, request.gradient))
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().expect("Objective worker panicked"))
                .collect::<Vec<_>>()
        });
        let n = indices.len();
        let mut result = Totals {
            loss: 0.0,
            correct: 0,
            top5: 0,
            rank: 0,
            gradient: vec![
                0.0;
                if request.gradient {
                    request.gains.len()
                } else {
                    0
                }
            ],
            diagonal: vec![
                0.0;
                if request.gradient {
                    request.gains.len()
                } else {
                    0
                }
            ],
        };
        for p in partials {
            result.loss += p.loss;
            result.correct += p.correct;
            result.top5 += p.top5;
            result.rank += p.rank;
            for (a, b) in result.gradient.iter_mut().zip(p.gradient) {
                *a += b / n as f64;
            }
            for (a, b) in result.diagonal.iter_mut().zip(p.diagonal) {
                *a += b / n as f64;
            }
        }
        json!({"records":n,"loss":result.loss/n as f64,"correct":result.correct,
            "accuracy":result.correct as f64/n as f64,"top5_accuracy":result.top5 as f64/n as f64,
            "mean_teacher_rank":result.rank as f64/n as f64,"gradient":result.gradient,
            "diagonal":result.diagonal,"seconds":start.elapsed().as_secs_f64()})
    }
}
fn main() -> Result<(), Box<dyn Error>> {
    let a: Vec<_> = std::env::args().collect();
    if a.len() != 7 {
        return Err(
            "Usage: synapse-objective GRAPH MODEL CACHE_DIR TEMPERATURE WORKERS MAX_TRAIN".into(),
        );
    }
    let graph = Graph::read(Path::new(&a[1]))?;
    let circuit: Circuit = serde_json::from_slice(&fs::read(&a[2])?)?;
    circuit.validate(&graph)?;
    let cached = Path::new(&a[3]);
    let temperature: f64 = a[4].parse()?;
    let workers: usize = a[5].parse()?;
    let max_train: usize = a[6].parse()?;
    if !temperature.is_finite() || temperature <= 0.0 || workers == 0 {
        return Err("Invalid settings".into());
    }
    let teachers: Vec<Teacher> = fs::read_to_string(cached.join("teachers.jsonl"))?
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;
    let mut spikes = fs::read(cached.join("spikes.bin"))?;
    let sources = circuit.source_neurons().len();
    let width = sources * circuit.time_bins;
    if spikes.len() != teachers.len() * width {
        return Err("Invalid cache length".into());
    }
    let mut rows = Vec::new();
    let mut train_count = 0;
    let mut selected = 0;
    for (i, r) in teachers.into_iter().enumerate() {
        if r.split != "train" && r.split != "validation" {
            return Err("Invalid split".into());
        }
        if r.split == "train" {
            if max_train > 0 && train_count >= max_train {
                continue;
            }
            train_count += 1;
        }
        let p = board::position_from_sfen(&r.sfen)?;
        let mut moves = board::Move32List::new();
        board::generate_legal_all_move32(&p, &mut moves);
        let labels: Vec<_> = moves
            .iter()
            .map(|&m| CompactMoveLabel::from_move32(m, p.turn()).unwrap().raw() as usize)
            .collect();
        let target = labels
            .iter()
            .position(|&l| l == r.target)
            .ok_or("Illegal teacher")?;
        rows.push(Row {
            labels,
            target,
            train: r.split == "train",
        });
        spikes.copy_within(i * width..(i + 1) * width, selected * width);
        selected += 1;
    }
    spikes.truncate(selected * width);
    if train_count == 0 || rows.iter().all(|r| r.train) {
        return Err("Both splits required".into());
    }
    let mut source_indices = vec![usize::MAX; graph.ids.len()];
    for (i, &s) in circuit.source_neurons().iter().enumerate() {
        source_indices[s] = i;
    }
    let dim = circuit.mbon.len() * circuit.time_bins;
    let mut source_edges = vec![Vec::new(); sources];
    for (edge, &pre) in circuit.pres.iter().enumerate() {
        source_edges[source_indices[pre]].push(edge);
    }
    let objective = Objective {
        spikes,
        rows,
        source_edges,
        groups: circuit.groups.clone(),
        weights: circuit
            .edges
            .iter()
            .map(|&e| graph.weights[e] as f64)
            .collect(),
        codes: (0..1496)
            .map(|l| {
                (0..dim)
                    .map(|j| circuit.code(l, j) / (dim as f64).sqrt() / temperature)
                    .collect()
            })
            .collect(),
        sources,
        bins: circuit.time_bins,
        outputs: circuit.mbon.len(),
        workers,
    };
    drop(graph);
    let mut out = io::BufWriter::new(io::stdout().lock());
    writeln!(
        out,
        "{}",
        json!({"ready":true,"train_records":train_count,"validation_records":objective.rows.len()-train_count,"parameters":circuit.gains.len()})
    )?;
    out.flush()?;
    for line in io::stdin().lock().lines() {
        let request: Request = serde_json::from_str(&line?)?;
        if request.gains.len() != circuit.gains.len()
            || request
                .gains
                .iter()
                .any(|x| !x.is_finite() || !(0.05..=3.0).contains(x))
            || !["train", "validation"].contains(&request.split.as_str())
        {
            return Err("Invalid objective request".into());
        }
        writeln!(out, "{}", objective.evaluate(&request))?;
        out.flush()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn objective_matches_circuit_and_finite_difference_with_empty_bins() {
        let graph = Graph {
            ids: vec![1, 2, 3],
            offsets: vec![0, 1, 2, 2],
            targets: vec![2, 2],
            weights: vec![3, 7],
        };
        let circuit: Circuit = serde_json::from_value(json!({
            "version":"kc-mbon-v2","kc":[0,1],"mbon":[2],"edges":[0,1],"pres":[0,1],
            "groups":[0,0],"gains":[1.25,0.75],"samples":0,"updates":0,"code_seed":73,
            "steps":400,"time_bins":4,"structured_code":true
        }))
        .unwrap();
        let other = (1..1496)
            .find(|&l| (0..4).any(|j| circuit.code(l, j) != circuit.code(0, j)))
            .unwrap();
        let counts = vec![2, 0, 0, 0, 5, 0, 0, 0, 0, 3, 1, 0];
        let temperature = 0.02;
        let objective = Objective {
            spikes: vec![2, 0, 0, 5, 0, 0, 3, 1],
            rows: vec![Row {
                labels: vec![0, other],
                target: 1,
                train: true,
            }],
            source_edges: vec![vec![0], vec![1]],
            groups: vec![0, 0],
            weights: vec![3., 7.],
            sources: 2,
            bins: 4,
            outputs: 1,
            workers: 1,
            codes: (0..1496)
                .map(|l| {
                    (0..4)
                        .map(|j| circuit.code(l, j) / 2. / temperature)
                        .collect()
                })
                .collect(),
        };
        let x = vec![1.25, 0.75];
        let result = objective.compute(&x, &[0], true);
        let scores = circuit.logits(&graph, &counts);
        let max = (scores[0] / temperature).max(scores[other] / temperature);
        let expected = max
            + ((scores[0] / temperature - max).exp() + (scores[other] / temperature - max).exp())
                .ln()
            - scores[other] / temperature;
        assert!((result.loss - expected).abs() < 1e-12);
        for i in 0..2 {
            let mut plus = x.clone();
            plus[i] += 1e-6;
            let mut minus = x.clone();
            minus[i] -= 1e-6;
            let difference = (objective.compute(&plus, &[0], false).loss
                - objective.compute(&minus, &[0], false).loss)
                / 2e-6;
            assert!((difference - result.gradient[i]).abs() < 1e-7);
        }
        let uniform = objective.compute(&[1., 1.], &[0], false);
        assert_eq!(uniform.loss, 2f64.ln());
        assert_eq!(uniform.correct, 0); // Lower compact label wins an exact tie.
    }
}
