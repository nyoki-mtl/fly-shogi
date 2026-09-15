//! Online full-connectome learning using only local eligibility and nonnegative DA.
use fly_shogi_lab::{
    dopamine::{Channel, depress, paired},
    lif::{self, Graph},
    plasticity::Circuit,
};
use rsshogi::{board, labels::policy::CompactMoveLabel};
use serde::Deserialize;
use serde_json::json;
use std::{error::Error, fs, io::Write, path::Path, time::Instant};

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
    #[serde(default)]
    teacher_policy: Vec<(usize, f64)>,
}
#[derive(Deserialize, serde::Serialize)]
struct Schedule {
    early_amplitude_factor: f64,
    anneal_samples: f64,
    checkpoint_every: usize,
    #[serde(default)]
    soft_bounds: bool,
    #[serde(default)]
    normalize_dopamine: bool,
    #[serde(default)]
    saturate_trace: bool,
    #[serde(default)]
    adapt_trace: bool,
    #[serde(default = "one")]
    reward_trials: usize,
    #[serde(default)]
    center_action_tag: bool,
    #[serde(default)]
    temporal_credit: bool,
    #[serde(default)]
    balanced_timing: bool,
    #[serde(default)]
    local_competition: bool,
    #[serde(default)]
    predict_reward: bool,
    #[serde(default)]
    resume_local: bool,
}
fn one() -> usize {
    1
}
fn save(path: &Path, value: &impl serde::Serialize) -> Result<(), Box<dyn Error>> {
    let temp = path.with_extension("tmp");
    fs::write(&temp, serde_json::to_vec(value)?)?;
    fs::rename(temp, path)?;
    Ok(())
}
fn main() -> Result<(), Box<dyn Error>> {
    let a: Vec<_> = std::env::args().collect();
    if !(9..=11).contains(&a.len()) {
        return Err(
            "Usage: train-dopamine GRAPH METADATA INITIAL CHANNELS TEACHERS RUN EPOCHS ETA [ltd|paired|policy-reward] [SCHEDULE_JSON]".into(),
        );
    }
    let g = Graph::read(Path::new(&a[1]))?;
    let pools: Pools = serde_json::from_slice(&fs::read(&a[2])?)?;
    let mut c: Circuit = serde_json::from_slice(&fs::read(&a[3])?)?;
    c.validate(&g)?;
    if c.upstream.is_some() || !c.sources.is_empty() {
        return Err("Requires KC-only plastic synapses".into());
    }
    let channels: Vec<Channel> = serde_json::from_slice(&fs::read(&a[4])?)?;
    if channels.is_empty()
        || channels.iter().any(|d| {
            d.targets.is_empty()
                || d.targets
                    .iter()
                    .any(|&(m, w)| m >= c.mbon.len() || !w.is_finite() || w <= 0.0)
        })
    {
        return Err("Invalid DAN targeting map".into());
    }
    let teachers: Vec<Teacher> = fs::read_to_string(&a[5])?
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<Vec<_>, _>>()?;
    let teachers: Vec<_> = teachers
        .into_iter()
        .filter(|t| t.split == "train")
        .collect();
    let run = Path::new(&a[6]);
    let epochs: usize = a[7].parse()?;
    let eta: f64 = a[8].parse()?;
    let mode = a.get(9).map(String::as_str).unwrap_or("ltd");
    if !["ltd", "paired", "policy-reward"].contains(&mode) {
        return Err("Invalid timing mode".into());
    }
    let reward_mode = mode == "policy-reward";
    let timing = mode != "ltd";
    let schedule: Schedule = if let Some(path) = a.get(10) {
        serde_json::from_slice(&fs::read(path)?)?
    } else {
        Schedule {
            early_amplitude_factor: 2.0,
            anneal_samples: 0.0,
            checkpoint_every: 2048,
            soft_bounds: false,
            normalize_dopamine: false,
            saturate_trace: false,
            adapt_trace: false,
            reward_trials: 1,
            center_action_tag: false,
            temporal_credit: false,
            balanced_timing: false,
            local_competition: false,
            predict_reward: false,
            resume_local: false,
        }
    };
    if !schedule.early_amplitude_factor.is_finite()
        || schedule.early_amplitude_factor <= 0.0
        || !schedule.anneal_samples.is_finite()
        || schedule.anneal_samples < 0.0
        || schedule.checkpoint_every == 0
        || schedule.reward_trials == 0
        || schedule.reward_trials > 64
        || (schedule.temporal_credit && schedule.adapt_trace)
        || (schedule.temporal_credit && !reward_mode)
        || (schedule.balanced_timing && !schedule.temporal_credit)
        || (schedule.local_competition && (!schedule.temporal_credit || schedule.balanced_timing))
    {
        return Err("Invalid stimulus schedule".into());
    }
    if teachers.is_empty() || epochs == 0 || !eta.is_finite() || eta <= 0.0 {
        return Err("Invalid settings".into());
    }
    let initial_samples = c.samples;
    if schedule.resume_local {
        let origin = Path::new(&a[3])
            .parent()
            .ok_or("Missing local training directory")?;
        let settings: serde_json::Value =
            serde_json::from_slice(&fs::read(origin.join("settings.json"))?)?;
        if settings["rule"] != "policy-reward" || schedule.adapt_trace {
            return Err("Resume requires a saved policy-reward checkpoint and no unsaved adaptive trace state".into());
        }
    } else if c.samples != 0 || c.gains.iter().any(|&w| w != 1.0) {
        return Err(
            "Requires neutral synapses unless resume_local verifies local training provenance"
                .into(),
        );
    }
    fs::create_dir(run)?;
    fs::copy(&a[3], run.join("initial.json"))?;
    fs::copy(&a[4], run.join("channels.json"))?;
    fs::copy(&a[5], run.join("teachers.jsonl"))?;
    save(
        &run.join("initialization.json"),
        &json!({"source":a[3],"resumed_local":schedule.resume_local,"initial_samples":initial_samples}),
    )?;
    save(
        &run.join("settings.json"),
        &json!({"rule":mode,"tau_ms":40,"eta":eta,"epochs":epochs,"teacher_interface":if reward_mode {"sampled-action code gated by scalar teacher-probability advantage"} else {"fixed-code preference averaged across time bins and DAN target contacts on an incorrect answer"}, "dopamine":"artificial pulse amplitude, not simulated DAN firing or biochemical concentration", "initial_gain":if schedule.resume_local {None} else {Some(1)},"potentiation":timing,"early_pulse_time_ms":-20,"late_pulse_time_ms":(c.steps-1) as f64*0.1,"feedback_replay":timing,"replay_computation":"exact reuse: identical reset state, stimulus seed and pre-update gains","schedule":schedule}),
    )?;
    if reward_mode {
        save(
            &run.join("reward-settings.json"),
            &json!({
                "reward":"teacher probability assigned to the sampled action",
                "baseline":if schedule.predict_reward {"fly choice probability of this action, interpreted as predicted teacher approval (model assumption)"} else {"1 / number of legal moves; uniform-policy expected reward"},
                "action_sampling_temperature":0.02,"uniform_exploration":0.1,
                "teacher_argmax_used_for_updates":false,
                "action_tag":"fixed readout code of the sampled action, projected onto DAN targets",
                "signed_advantage":"controls nonnegative early/late pulses; not negative dopamine concentration",
                "cross_entropy":"diagnostic only, not differentiated or used for updates"
                ,"reward_trials":schedule.reward_trials,
                "center_action_tag":schedule.center_action_tag,
                "temporal_credit":schedule.temporal_credit,
                "repeated_choices":"independent samples from unchanged fly policy; average scalar-reward updates before changing gains"
            }),
        )?;
    }
    let mut dense = c.dense(&g);
    let mut dopamine_mass = vec![0.0f64; c.mbon.len()];
    let mut mean_late = vec![0.0f64; g.ids.len()];
    let mut mean_early = vec![0.0f64; g.ids.len()];
    for channel in &channels {
        for &(m, w) in &channel.targets {
            dopamine_mass[m] += w;
        }
    }
    let started = Instant::now();
    let timing_scale: Vec<_> = (0..c.time_bins)
        .map(|bin| {
            let width = (c.steps - c.observation_start) / c.time_bins;
            let midpoint = (c.observation_start + bin * width) as f64 + (width - 1) as f64 / 2.0;
            ((20.0 + 2.0 * midpoint * 0.1 - (c.steps - 1) as f64 * 0.1) / 40.0).exp()
        })
        .collect();
    for epoch in 0..epochs {
        let mut order: Vec<_> = (0..teachers.len()).collect();
        for i in (1..order.len()).rev() {
            let j = lif::mix(
                241 ^ (((epoch + initial_samples / teachers.len()) as u64) << 32) ^ i as u64,
            ) as usize
                % (i + 1);
            order.swap(i, j);
        }
        let mut correct = 0;
        let mut loss = 0.0;
        let mut pulse_count = 0;
        let mut reward_sum = 0.0;
        for (step, &index) in order.iter().enumerate() {
            let effective_eta = eta
                / if schedule.anneal_samples > 0.0 {
                    1.0 + c.samples as f64 / schedule.anneal_samples
                } else {
                    1.0
                };
            let r = &teachers[index];
            let p = board::position_from_sfen(&r.sfen)?;
            let mut moves = board::Move32List::new();
            board::generate_legal_all_move32(&p, &mut moves);
            let labels: Vec<_> = moves
                .iter()
                .map(|&m| CompactMoveLabel::from_move32(m, p.turn()).unwrap().raw() as usize)
                .collect();
            if !labels.contains(&r.target) {
                return Err("Illegal teacher".into());
            }
            let response = c.run(&g, &pools.input_pool, &p, r.seed, &dense)?;
            let scores = c.logits(&g, response.policy_counts());
            let greedy = *labels
                .iter()
                .max_by(|&&x, &&y| scores[x].total_cmp(&scores[y]).then(y.cmp(&x)))
                .unwrap();
            let max = scores[greedy] / 0.02;
            let mut weights: Vec<_> = labels
                .iter()
                .map(|&l| (scores[l] / 0.02 - max).exp())
                .collect();
            let total: f64 = weights.iter().sum();
            // 10% uniform exploration is part of the training apparatus.
            for w in &mut weights {
                *w = 0.9 * *w / total + 0.1 / labels.len() as f64;
            }
            let choose = |trial: usize| {
                let mut draw = (lif::mix(
                    827 ^ c.samples as u64 ^ (trial as u64).wrapping_mul(0xd1342543de82ef95),
                ) >> 11) as f64
                    / 9007199254740992.0;
                let mut selected = *labels.last().unwrap();
                for (&l, &w) in labels.iter().zip(&weights) {
                    draw -= w;
                    if draw <= 0.0 {
                        selected = l;
                        break;
                    }
                }
                selected
            };
            let chosen = if reward_mode { choose(0) } else { greedy };
            let bins = if schedule.temporal_credit {
                c.time_bins
            } else {
                1
            };
            let code = |label: usize, bin: usize, m: usize| {
                if schedule.temporal_credit {
                    c.code(label, bin * c.mbon.len() + m)
                } else {
                    (0..c.time_bins)
                        .map(|b| c.code(label, b * c.mbon.len() + m))
                        .sum::<f64>()
                        / c.time_bins as f64
                }
            };
            let mut preference = vec![0.0; bins * c.mbon.len()];
            if reward_mode {
                let mut probabilities = vec![None; 1496];
                for &(label, p) in &r.teacher_policy {
                    if label >= 1496
                        || !p.is_finite()
                        || !(0.0..=1.0).contains(&p)
                        || probabilities[label].is_some()
                    {
                        return Err("Invalid teacher policy".into());
                    }
                    probabilities[label] = Some(p);
                }
                if r.teacher_policy.len() != labels.len()
                    || labels.iter().any(|&l| probabilities[l].is_none())
                    || (r.teacher_policy.iter().map(|&(_, p)| p).sum::<f64>() - 1.0).abs() > 1e-6
                {
                    return Err("Teacher policy does not match legal moves".into());
                }
                let expected: Vec<_> = (0..preference.len())
                    .map(|k| {
                        if schedule.center_action_tag {
                            labels
                                .iter()
                                .zip(&weights)
                                .map(|(&l, &w)| w * code(l, k / c.mbon.len(), k % c.mbon.len()))
                                .sum::<f64>()
                        } else {
                            0.0
                        }
                    })
                    .collect();
                for trial in 0..schedule.reward_trials {
                    let action = choose(trial);
                    let reward = probabilities[action].unwrap();
                    reward_sum += reward / schedule.reward_trials as f64;
                    let expected_reward = if schedule.predict_reward {
                        weights[labels.iter().position(|&l| l == action).unwrap()]
                    } else {
                        1.0 / labels.len() as f64
                    };
                    let advantage = reward - expected_reward;
                    for (k, v) in preference.iter_mut().enumerate() {
                        *v += (code(action, k / c.mbon.len(), k % c.mbon.len()) - expected[k])
                            * advantage
                            / schedule.reward_trials as f64;
                    }
                }
            } else {
                for (k, v) in preference.iter_mut().enumerate() {
                    *v = (code(r.target, k / c.mbon.len(), k % c.mbon.len())
                        - code(chosen, k / c.mbon.len(), k % c.mbon.len()))
                        / 2.0;
                }
            }
            loss += labels
                .iter()
                .map(|&l| (scores[l] / 0.02 - max).exp())
                .sum::<f64>()
                .ln()
                + max
                - scores[r.target] / 0.02;
            correct += usize::from(chosen == r.target);
            if reward_mode || chosen != r.target {
                // Teacher equipment addresses anatomical channels with a fixed code.
                // No loss differentiation through LIF; optional local competition is below.
                let mut da = vec![0.0; preference.len()];
                let mut early_da = vec![0.0; preference.len()];
                for bin in 0..bins {
                    let offset = bin * c.mbon.len();
                    for channel in &channels {
                        let preference = channel
                            .targets
                            .iter()
                            .map(|&(m, w)| preference[offset + m] * w)
                            .sum::<f64>();
                        let pulse = preference.max(0.0);
                        if pulse > 0.0 || (timing && preference < 0.0) {
                            pulse_count += 1;
                        }
                        for &(m, w) in &channel.targets {
                            da[offset + m] += pulse * w;
                            early_da[offset + m] += (-preference).max(0.0) * w;
                        }
                    }
                }
                // Replay has identical reset state, stimulus and unchanged gains.
                // DA affects only the endpoint update, so reusing these spikes is exact.
                // No activity is reused across different trials or changed weights.
                let trace = &response;
                // Optional model assumption: compete for a local MBON plasticity budget.
                // Only existing afferents of that MBON enter these denominators.
                let mut local_late = vec![0.0; bins * c.mbon.len()];
                let mut local_early = vec![0.0; bins * c.mbon.len()];
                if schedule.local_competition {
                    for bin in 0..bins {
                        for i in 0..c.edges.len() {
                            let k = bin * c.mbon.len() + c.groups[i];
                            let pre = bin * g.ids.len() + c.pres[i];
                            let contacts = g.weights[c.edges[i]] as f64;
                            local_late[k] += contacts * trace.temporal_eligibility[pre];
                            local_early[k] += contacts * trace.temporal_preceding_overlap[pre];
                        }
                    }
                }
                for i in 0..c.edges.len() {
                    let mut delta = 0.0;
                    for bin in 0..bins {
                        let group = bin * c.mbon.len() + c.groups[i];
                        let denom = if schedule.normalize_dopamine {
                            dopamine_mass[c.groups[i]].max(1.0)
                        } else {
                            1.0
                        };
                        let mut late_trace = if schedule.temporal_credit {
                            trace.temporal_eligibility[bin * g.ids.len() + c.pres[i]]
                        } else {
                            trace.eligibility[c.pres[i]]
                        };
                        let mut early_trace = if schedule.temporal_credit {
                            trace.temporal_preceding_overlap[bin * g.ids.len() + c.pres[i]]
                        } else {
                            trace.preceding_dopamine_overlap[c.pres[i]]
                        };
                        if schedule.balanced_timing {
                            // Known stimulus timing, not a fitted parameter or error gradient.
                            // Match early/late decay at the midpoint of this observation bin.
                            early_trace *= timing_scale[bin];
                        }
                        if schedule.local_competition {
                            let contacts = g.weights[c.edges[i]] as f64;
                            late_trace = if local_late[group] > 0.0 {
                                late_trace * contacts / local_late[group]
                            } else {
                                0.0
                            };
                            early_trace = if local_early[group] > 0.0 {
                                early_trace * contacts / local_early[group]
                            } else {
                                0.0
                            };
                        }
                        if schedule.adapt_trace {
                            late_trace = (late_trace - mean_late[c.pres[i]]).max(0.0);
                            early_trace = (early_trace - mean_early[c.pres[i]]).max(0.0);
                        }
                        if schedule.saturate_trace {
                            late_trace /= 1.0 + late_trace;
                            early_trace /= 1.0 + early_trace;
                        }
                        // Local saturation hypothesis; no update without activity and DA.
                        if schedule.soft_bounds {
                            late_trace *= (c.gains[i] as f64 - 0.05) / 0.95;
                            early_trace *= (3.0 - c.gains[i] as f64) / 2.0;
                        }
                        if schedule.temporal_credit {
                            // Each timing tag gates the trace of the same time bin.
                            // Sum local contributions, then enforce bounds once.
                            delta += effective_eta
                                * (early_trace
                                    * schedule.early_amplitude_factor
                                    * (early_da[group] / denom).min(1.0)
                                    - late_trace * (da[group] / denom).min(1.0));
                        } else {
                            c.gains[i] = if timing {
                                paired(
                                    c.gains[i],
                                    late_trace,
                                    early_trace,
                                    (da[c.groups[i]] / denom).min(1.0),
                                    schedule.early_amplitude_factor
                                        * (early_da[c.groups[i]] / denom).min(1.0),
                                    effective_eta,
                                )
                            } else {
                                depress(
                                    c.gains[i],
                                    trace.eligibility[c.pres[i]],
                                    da[c.groups[i]].min(1.0),
                                    effective_eta,
                                )
                            };
                        }
                    }
                    if schedule.temporal_credit {
                        c.gains[i] = (c.gains[i] as f64 + delta).clamp(0.05, 3.0) as f32;
                    }
                    dense[c.edges[i]] = c.gains[i];
                }
                c.updates += 1;
            }
            if schedule.adapt_trace {
                for &node in &c.kc {
                    mean_late[node] += (response.eligibility[node] - mean_late[node]) / 128.0;
                    mean_early[node] +=
                        (response.preceding_dopamine_overlap[node] - mean_early[node]) / 128.0;
                }
            }
            c.samples += 1;
            if c.samples % schedule.checkpoint_every == 0 {
                save(&run.join(format!("samples-{:07}.json", c.samples)), &c)?;
            }
            if (step + 1) % 128 == 0 || step + 1 == order.len() {
                let report = json!({"stage":"training","epoch":epoch+1,"step":step+1,"samples":c.samples,"online_accuracy":correct as f64/(step+1) as f64,"online_loss":loss/(step+1) as f64,"mean_teacher_probability_reward":if reward_mode {Some(reward_sum/(step+1) as f64)} else {None},"dan_pulses":pulse_count,"changed_edges":c.gains.iter().filter(|&&w|w!=1.0).count(),"mean_gain":c.gains.iter().map(|&w|w as f64).sum::<f64>()/c.gains.len() as f64,"seconds":started.elapsed().as_secs_f64()});
                save(&run.join("progress.json"), &report)?;
                writeln!(
                    fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(run.join("progress.jsonl"))?,
                    "{report}"
                )?;
                println!("{report}");
                if run.join("STOP").exists() {
                    save(&run.join("stopped.json"), &c)?;
                    save(
                        &run.join("stop.json"),
                        &json!({"status":"stopped_by_request","samples":c.samples,"seconds":started.elapsed().as_secs_f64()}),
                    )?;
                    return Ok(());
                }
            }
        }
        save(&run.join(format!("epoch-{:03}.json", epoch + 1)), &c)?;
    }
    save(&run.join("final.json"), &c)?;
    save(
        &run.join("complete.json"),
        &json!({"samples":c.samples,"initial_samples":initial_samples,"run_samples":c.samples-initial_samples,"reward_observations":if reward_mode {Some((c.samples-initial_samples)*schedule.reward_trials)} else {None},"seconds":started.elapsed().as_secs_f64()}),
    )?;
    Ok(())
}
