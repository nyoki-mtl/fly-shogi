//! Small held-out teacher check; never updates the circuit.
use fly_shogi_lab::{lif::Graph, plasticity::Circuit};
use rsshogi::{board, labels::policy::CompactMoveLabel};
use serde::Deserialize;
use serde_json::json;
use std::{error::Error, fs, path::Path, time::Instant};

#[derive(Deserialize)]
struct Pools {
    input_pool: Vec<usize>,
}
#[derive(Deserialize)]
struct Teacher {
    sfen: String,
    target: usize,
    seed: u64,
    split: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let a: Vec<_> = std::env::args().collect();
    if a.len() != 7 {
        return Err("Usage: evaluate-circuit GRAPH METADATA MODEL TEACHERS COUNT OUTPUT".into());
    }
    let count: usize = a[5].parse()?;
    if count == 0 {
        return Err("COUNT must be positive".into());
    }
    let g = Graph::read(Path::new(&a[1]))?;
    let pool: Pools = serde_json::from_slice(&fs::read(&a[2])?)?;
    let model_bytes = fs::read(&a[3])?;
    let c: Circuit = serde_json::from_slice(&model_bytes)?;
    c.validate(&g)?;
    let records: Vec<Teacher> = fs::read_to_string(&a[4])?
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;
    let held: Vec<_> = records
        .iter()
        .filter(|r| r.split == "validation")
        .take(count)
        .collect();
    if held.len() != count {
        return Err("Not enough validation records".into());
    }
    let output = Path::new(&a[6]);
    if output.exists() || output.with_extension("model.json").exists() {
        return Err("Choose a new OUTPUT to preserve earlier checks".into());
    }
    // Retain the exact checkpoint even while training replaces the live file.
    fs::write(output.with_extension("model.json"), model_bytes)?;
    let dense = c.dense(&g);
    let start = Instant::now();
    let mut results = Vec::new();
    for chunk in held.chunks(4) {
        let batch = std::thread::scope(|s| {
            let handles: Vec<_>=chunk.iter().map(|r| {
                let (g,c,pool,dense)=(&g,&c,&pool,&dense);
                s.spawn(move || -> Result<_,String> {
                    let p=board::position_from_sfen(&r.sfen).map_err(|e|e.to_string())?;
                    let response=c.run(g,&pool.input_pool,&p,r.seed,dense)?;
                    let scores=c.logits(g,response.policy_counts());
                    let mut moves=board::Move32List::new();
                    board::generate_legal_all_move32(&p,&mut moves);
                    let labels: Vec<_>=moves.iter().map(|&m|CompactMoveLabel::from_move32(m,p.turn()).unwrap().raw() as usize).collect();
                    if !labels.contains(&r.target) {return Err("Illegal teacher".into());}
                    let chosen=*labels.iter().max_by(|&&x,&&y|scores[x].total_cmp(&scores[y]).then(y.cmp(&x))).ok_or("No legal moves")?;
                    let tied=labels.iter().filter(|&&l|(scores[l]-scores[chosen]).abs()<1e-12).count();
                    let credit=if (scores[r.target]-scores[chosen]).abs()<1e-12 {1.0/tied as f64}else{0.0};
                    let rank=1+labels.iter().filter(|&&l| scores[l]>scores[r.target] || (scores[l]==scores[r.target] && l<r.target)).count();
                    let competitor=labels.iter().copied().filter(|&l|l!=r.target).map(|l|scores[l]).reduce(f64::max);
                    let margin=competitor.map(|s|scores[r.target]-s);
                    let max_logit=labels.iter().map(|&l|scores[l]/0.02).fold(f64::NEG_INFINITY,f64::max);
                    let cross_entropy=max_logit+labels.iter().map(|&l|(scores[l]/0.02-max_logit).exp()).sum::<f64>().ln()-scores[r.target]/0.02;
                    // At gain=1, activity() is exactly zero for every group, so all
                    // legal labels tie. Its uniform tie expectation needs no LIF run.
                    Ok(json!({"sfen":r.sfen,"seed":r.seed,"target":r.target,"chosen":chosen,"correct":chosen==r.target,"tie_credit":credit,"uniform_legal_expected":1.0/labels.len() as f64,"legal_count":labels.len(),"teacher_rank":rank,"top5":rank<=5,"teacher_margin":margin,"teacher_score":scores[r.target],"cross_entropy":cross_entropy}))
                })
            }).collect();
            handles
                .into_iter()
                .map(|h| {
                    h.join()
                        .map_err(|_| "Evaluation thread panicked".to_string())?
                })
                .collect::<Result<Vec<_>, String>>()
        })?;
        results.extend(batch);
        eprintln!("evaluated {}/{}", results.len(), count);
    }
    let correct = results.iter().filter(|r| r["correct"] == true).count();
    let credit: f64 = results
        .iter()
        .map(|r| r["tie_credit"].as_f64().unwrap())
        .sum();
    let baseline: f64 = results
        .iter()
        .map(|r| r["uniform_legal_expected"].as_f64().unwrap())
        .sum();
    let mean_rank = results
        .iter()
        .map(|r| r["teacher_rank"].as_u64().unwrap() as f64)
        .sum::<f64>()
        / count as f64;
    let top5 = results.iter().filter(|r| r["top5"] == true).count();
    let margins: Vec<_> = results
        .iter()
        .filter_map(|r| r["teacher_margin"].as_f64())
        .collect();
    let mean_margin = if margins.is_empty() {
        None
    } else {
        Some(margins.iter().sum::<f64>() / margins.len() as f64)
    };
    let loss = results
        .iter()
        .map(|r| r["cross_entropy"].as_f64().unwrap())
        .sum::<f64>()
        / count as f64;
    let report = json!({"checkpoint_samples":c.samples,"positions":count,"correct":correct,"accuracy":correct as f64/count as f64,"tie_aware_accuracy":credit/count as f64,"initial_uniform_tie_expected_accuracy":baseline/count as f64,"initial_policy_has_no_preference":true,"mean_teacher_rank":mean_rank,"top5_accuracy":top5 as f64/count as f64,"mean_teacher_margin":mean_margin,"cross_entropy":loss,"cross_entropy_temperature":0.02,"elapsed_seconds":start.elapsed().as_secs_f64(),"records":results});
    fs::write(output, serde_json::to_vec_pretty(&report)?)?;
    println!(
        "samples={} heldout={} correct={} accuracy={:.4} initial_uniform_expectation={:.4}",
        c.samples,
        count,
        correct,
        credit / count as f64,
        baseline / count as f64
    );
    Ok(())
}
