//! Persistent JSON-lines worker for the local playable demo.
use fly_shogi_lab::{encoding, lif::Graph, plasticity::Circuit};
use rsshogi::{
    board,
    labels::policy::CompactMoveLabel,
    types::{Color, HandPiece, Piece, RepetitionState, Square},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    error::Error,
    fs,
    io::{self, BufRead, Write},
    path::Path,
    time::Instant,
};

#[derive(Deserialize)]
struct Pools {
    input_pool: Vec<usize>,
}
#[derive(Deserialize)]
struct AtlasNode {
    index: usize,
}
#[derive(Deserialize)]
struct Atlas {
    nodes: Vec<AtlasNode>,
}
#[derive(Deserialize)]
struct Request {
    action: String,
    #[serde(default)]
    sfen: String,
    #[serde(default)]
    moves: Vec<String>,
    #[serde(default = "default_seed")]
    seed: u64,
    #[serde(default)]
    move_usi: Option<String>,
}
fn default_seed() -> u64 {
    101
}

fn legal(p: &board::Position) -> board::Move32List {
    let mut moves = board::Move32List::new();
    board::generate_legal_all_move32(p, &mut moves);
    moves
}

fn terminal(p: &board::Position, length: usize) -> Option<String> {
    if p.is_repetition(3) {
        let winner = match p.repetition_state() {
            RepetitionState::Draw => return Some("Draw by repetition".into()),
            RepetitionState::Win => Some(p.turn()),
            RepetitionState::Lose => Some(!p.turn()),
            _ => None,
        };
        if let Some(winner) = winner {
            let player = if winner == Color::BLACK {
                "Human"
            } else {
                "Fly Meijin"
            };
            return Some(format!("{player} wins by perpetual-check foul"));
        }
    }
    if legal(p).is_empty() {
        return Some(
            if p.is_in_check() {
                "Checkmate"
            } else {
                "No legal moves"
            }
            .into(),
        );
    }
    if length >= 256 {
        return Some("Game ended after 256 moves".into());
    }
    None
}

fn snapshot(p: &board::Position, moves: &[String]) -> Value {
    let cells:Vec<_>=Square::iter().filter_map(|sq| {
        let piece=p.piece_on(sq);
        (piece!=Piece::NONE).then(||json!({"square":sq.to_usi(),"piece":piece.piece_type().to_string(),"side":if piece.color()==Color::BLACK{"b"}else{"w"}}))
    }).collect();
    let hands:Vec<_>=[Color::BLACK,Color::WHITE].iter().map(|&c| {
        HandPiece::iter().map(|hp|json!({"piece":hp.to_piece_type().to_string(),"count":p.hand(c).count(hp)})).collect::<Vec<_>>()
    }).collect();
    let end = terminal(p, moves.len());
    let labels: Vec<_> = if end.is_none() {
        legal(p)
            .iter()
            .map(|&m| {
                let compact = CompactMoveLabel::from_move32(m, p.turn()).unwrap();
                (compact.raw(), compact.expand().raw(), m.to_string())
            })
            .collect()
    } else {
        vec![]
    };
    json!({"sfen":p.to_sfen(None),"turn":if p.turn()==Color::BLACK{"b"}else{"w"},"cells":cells,"hands":hands,"legal_labels":labels,
        "legal":if end.is_none(){legal(p).iter().map(|m|m.to_string()).collect::<Vec<_>>()}else{vec![]},
        "moves":moves,"in_check":p.is_in_check(),"terminal":end})
}

fn respond(
    req: Request,
    graph: &Graph,
    pools: &Pools,
    circuit: &Circuit,
    gains: &[f32],
    atlas: &Atlas,
) -> Result<Value, Box<dyn Error>> {
    if req.moves.len() > 256 {
        return Err("Move limit exceeded".into());
    }
    let mut p = if req.sfen.is_empty() || req.sfen == "startpos" {
        board::hirate_position()
    } else {
        board::position_from_sfen(&req.sfen)?
    };
    let mut history = Vec::new();
    for usi in &req.moves {
        if terminal(&p, history.len()).is_some() {
            return Err("The game has already ended".into());
        }
        let list = legal(&p);
        let mv = *list
            .iter()
            .find(|m| m.to_string() == *usi)
            .ok_or("Illegal move")?;
        p.apply_move32(mv);
        history.push(usi.clone());
    }
    if req.action == "state" {
        return Ok(json!({"state":snapshot(&p,&history)}));
    }
    if req.action == "play" {
        if p.turn() != Color::BLACK || terminal(&p, history.len()).is_some() {
            return Err("It is not Human's turn".into());
        }
        let usi = req.move_usi.as_deref().ok_or("Missing move")?;
        let list = legal(&p);
        let mv = *list
            .iter()
            .find(|m| m.to_string() == usi)
            .ok_or("Illegal move")?;
        p.apply_move32(mv);
        history.push(usi.to_string());
        return Ok(json!({"state":snapshot(&p,&history)}));
    }
    if req.action != "think" {
        return Err("Unknown action".into());
    }
    if terminal(&p, history.len()).is_some() {
        return Ok(json!({"state":snapshot(&p,&history)}));
    }
    if p.turn() != Color::WHITE {
        return Err("Fly Meijin plays the second side".into());
    }
    let began = Instant::now();
    let active_features = if circuit.categorical_input {
        encoding::encode_categorical(&p)
            .iter()
            .filter(|&&b| b != 0)
            .count()
    } else {
        encoding::encode(&p).iter().filter(|&&b| b != 0).count()
    };
    let result = circuit.run(graph, &pools.input_pool, &p, req.seed, gains)?;
    let scores = circuit.logits(graph, result.policy_counts());
    let mut ranked: Vec<_> = legal(&p)
        .iter()
        .map(|&m| {
            let label = CompactMoveLabel::from_move32(m, p.turn()).unwrap().raw() as usize;
            (label, m, scores[label])
        })
        .collect();
    ranked.sort_by(|a, b| b.2.total_cmp(&a.2).then(a.0.cmp(&b.0)));
    let norm = result.output_z.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm <= 1e-8 || (ranked.len() > 1 && ranked[0].2 - ranked.last().unwrap().2 <= 1e-12) {
        return Ok(
            json!({"state":snapshot(&p,&history),"notice":"No move could be selected from neural activity. Start a new game."}),
        );
    }
    let chosen = ranked[0].1;
    let total: f64 = ranked
        .iter()
        .map(|r| ((r.2 - ranked[0].2) / 0.02).exp())
        .sum();
    let predicted_reward = 0.9 / total + 0.1 / ranked.len() as f64;
    let decision = snapshot(&p, &history);
    let candidates: Vec<_> = ranked
        .iter()
        .take(5)
        .map(|r| json!({"move":r.1.to_string(),"score":r.2}))
        .collect();
    let mut neurons: Vec<_> = circuit
        .mbon
        .iter()
        .map(|&i| (graph.ids[i], result.counts[i]))
        .collect();
    neurons.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let bins: Vec<_> = result
        .policy_counts()
        .chunks(graph.ids.len())
        .map(|counts| {
            atlas
                .nodes
                .iter()
                .map(|node| counts[node.index])
                .collect::<Vec<_>>()
        })
        .collect();
    let population: Vec<u64> = result
        .policy_counts()
        .chunks(graph.ids.len())
        .map(|counts| counts.iter().map(|&c| c as u64).sum())
        .collect();
    let telemetry = json!({"elapsed_ms":began.elapsed().as_secs_f64()*1000.0,"simulation_ms":circuit.steps as f64*0.1,
        "brain_activity":bins,"population_activity":population,"observation_start_ms":circuit.observation_start as f64*0.1,
        "neurons":graph.ids.len(),"edges":graph.targets.len(),"active_features":active_features,
        "learning":circuit.info(),"spiking_neurons":result.counts.iter().filter(|&&c|c>0).count(),"output_spikes":circuit.mbon.iter().map(|&i|result.counts[i] as u64).sum::<u64>(),
        "output_active":circuit.mbon.iter().filter(|&&i|result.counts[i]>0).count(),"candidates":candidates,
        "top_neurons":neurons.iter().take(24).map(|&(id,count)|json!({"id":id.to_string(),"count":count})).collect::<Vec<_>>()});
    let usi = chosen.to_string();
    p.apply_move32(chosen);
    history.push(usi.clone());
    Ok(
        json!({"state":snapshot(&p,&history),"move":usi,"telemetry":telemetry,"decision":decision,"predicted_reward":predicted_reward}),
    )
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 5 {
        return Err("Usage: fly-shogi-demo GRAPH METADATA CIRCUIT ATLAS".into());
    }
    let graph = Graph::read(Path::new(&args[1]))?;
    let atlas: Atlas = serde_json::from_slice(&fs::read(&args[4])?)?;
    if atlas.nodes.iter().any(|node| node.index >= graph.ids.len()) {
        return Err("Atlas index outside graph".into());
    }
    let pools: Pools = serde_json::from_slice(&fs::read(&args[2])?)?;
    let model_path = Path::new(&args[3]);
    let mut circuit: Circuit = serde_json::from_slice(&fs::read(model_path)?)?;
    circuit.validate(&graph)?;
    let mut gains = circuit.dense(&graph);
    let mut stamp = fs::metadata(model_path)?.modified()?;
    println!(
        "{}",
        json!({"ready":true,"neurons":graph.ids.len(),"edges":graph.targets.len()})
    );
    io::stdout().flush()?;
    for line in io::stdin().lock().lines() {
        let line = line?;
        let next = fs::metadata(model_path)?.modified()?;
        if next != stamp {
            let replacement: Circuit = serde_json::from_slice(&fs::read(model_path)?)?;
            replacement.validate(&graph)?;
            gains = replacement.dense(&graph);
            circuit = replacement;
            stamp = next;
        }
        let response = serde_json::from_str::<Request>(&line)
            .map_err(|e| e.into())
            .and_then(|r| respond(r, &graph, &pools, &circuit, &gains, &atlas));
        println!(
            "{}",
            match response {
                Ok(mut value) => {
                    value["learning"] = circuit.info();
                    value
                }
                Err(error) => json!({"error":error.to_string()}),
            }
        );
        io::stdout().flush()?;
    }
    Ok(())
}

#[cfg(test)]
mod repetition_tests {
    use super::*;

    fn play_cycle(sfen: &str, cycle: [&str; 4], expected: &str) {
        let mut p = board::position_from_sfen(sfen).unwrap();
        let mut moves = Vec::new();
        for _ in 0..3 {
            for usi in cycle {
                assert_eq!(terminal(&p, moves.len()), None);
                let mv = *legal(&p)
                    .iter()
                    .find(|m| m.to_string() == usi)
                    .expect("legal fixture move");
                p.apply_move32(mv);
                moves.push(usi.to_owned());
            }
        }
        let state = snapshot(&p, &moves);
        assert_eq!(state["terminal"], expected);
        assert_eq!(state["legal"], json!([]));
        assert_eq!(state["legal_labels"], json!([]));
    }

    #[test]
    fn ordinary_repetition_is_a_draw_on_fourth_occurrence() {
        play_cycle(
            &board::hirate_position().to_sfen(None),
            ["2h3h", "8b7b", "3h2h", "7b8b"],
            "Draw by repetition",
        );
    }

    #[test]
    fn human_perpetual_check_loses_from_either_turn() {
        play_cycle(
            "4k4/9/5R3/9/9/9/9/9/4K4 b - 1",
            ["4c5c", "5a4a", "5c4c", "4a5a"],
            "Fly Meijin wins by perpetual-check foul",
        );
        play_cycle(
            "4k4/9/4R4/9/9/9/9/9/4K4 w - 1",
            ["5a4a", "5c4c", "4a5a", "4c5c"],
            "Fly Meijin wins by perpetual-check foul",
        );
    }

    #[test]
    fn fly_perpetual_check_loses_from_either_turn() {
        play_cycle(
            "4k4/9/9/9/9/9/3r5/9/4K4 w - 1",
            ["6g5g", "5i6i", "5g6g", "6i5i"],
            "Human wins by perpetual-check foul",
        );
        play_cycle(
            "4k4/9/9/9/9/9/4r4/9/4K4 b - 1",
            ["5i6i", "5g6g", "6i5i", "6g5g"],
            "Human wins by perpetual-check foul",
        );
    }
}
