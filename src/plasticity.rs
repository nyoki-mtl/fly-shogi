//! Learning resides on existing synapses into MBONs. The policy code is fixed.
use crate::lif::{self, Graph};
use serde::{Deserialize, Serialize};
#[derive(Deserialize, Serialize)]
pub struct Upstream {
    pub edges: Vec<usize>,
    pub pres: Vec<usize>,
    pub groups: Vec<usize>,
    pub gains: Vec<f32>,
}
#[derive(Deserialize, Serialize)]
pub struct Circuit {
    pub version: String,
    pub kc: Vec<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<usize>,
    pub mbon: Vec<usize>,
    pub edges: Vec<usize>,
    pub pres: Vec<usize>,
    pub groups: Vec<usize>,
    pub gains: Vec<f32>,
    pub samples: usize,
    pub updates: usize,
    pub code_seed: u64,
    #[serde(default = "default_steps")]
    pub steps: usize,
    #[serde(default = "default_time_bins")]
    pub time_bins: usize,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub observation_start: usize,
    #[serde(default, skip_serializing_if = "is_false")]
    pub categorical_input: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub kc_input: bool,
    #[serde(default = "default_input_replicas", skip_serializing_if = "is_one")]
    pub kc_replicas: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kc_afferents: Vec<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream: Option<Upstream>,
    #[serde(default)]
    pub structured_code: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn margin_update_improves_teacher_score_with_fixed_spikes() {
        let g = Graph {
            ids: vec![1, 2, 3],
            offsets: vec![0, 1, 2, 2],
            targets: vec![2, 2],
            weights: vec![3, 7],
        };
        let mut c = Circuit {
            version: "kc-mbon-v2".into(),
            sources: vec![],
            kc: vec![0, 1],
            mbon: vec![2],
            edges: vec![0, 1],
            pres: vec![0, 1],
            groups: vec![0, 0],
            gains: vec![1.0, 1.0],
            samples: 0,
            updates: 0,
            code_seed: 73,
            steps: 400,
            time_bins: 1,
            observation_start: 0,
            categorical_input: false,
            kc_input: false,
            kc_replicas: 1,
            kc_afferents: vec![],
            upstream: None,
            structured_code: false,
        };
        c.validate(&g).unwrap();
        let wrong = (1..1496).find(|&l| c.code(l, 0) != c.code(0, 0)).unwrap();
        let counts = vec![2, 5, 0];
        assert_eq!(c.logits(&g, &counts)[0], 0.0);
        assert!(c.teach(&g, &counts, 0, wrong, 0.02) > 0);
        let scores = c.logits(&g, &counts);
        assert!((scores[0] - scores[wrong] - 0.02).abs() < 1e-6);
        // A correct but low-margin prediction must still be reinforced.
        c.gains.fill(1.0);
        c.teach(&g, &counts, 0, wrong, 0.005);
        assert!(c.logits(&g, &counts)[0] > c.logits(&g, &counts)[wrong]);
        assert!(c.teach_legal(&g, &counts, 0, &[0, wrong], 0.02) > 0);
        let scores = c.logits(&g, &counts);
        assert!((scores[0] - scores[wrong] - 0.02).abs() < 1e-6);
        let before = c.gains.clone();
        assert_eq!(c.teach_legal(&g, &counts, 0, &[0], 0.02), 0);
        assert_eq!(before, c.gains);
        c.validate(&g).unwrap();
        c.time_bins = 4;
        c.gains.fill(1.0);
        let counts = vec![2, 0, 0, 0, 5, 0, 0, 0, 0, 3, 1, 0];
        // Unequal windows and a silent window exercise shared-gain gradients.
        let wrong = (1..1496)
            .find(|&l| (0..4).all(|b| c.code(l, b) == -c.code(0, b)))
            .unwrap();
        assert!(c.teach(&g, &counts, 0, wrong, 0.02) > 0);
        let scores = c.logits(&g, &counts);
        assert!((scores[0] - scores[wrong] - 0.02).abs() < 1e-6);
    }

    #[test]
    fn structured_code_shares_move_attributes_without_merging_labels() {
        use rsshogi::{
            labels::policy::CompactMoveLabel,
            types::{Color, Move},
        };
        let mut c: Circuit = serde_json::from_value(serde_json::json!({
            "version":"kc-mbon-v2", "kc":[], "mbon":[], "edges":[],
            "pres":[], "groups":[], "gains":[], "samples":0, "updates":0,
            "code_seed":73
        }))
        .unwrap();
        assert!(!c.structured_code);
        c.structured_code = true;
        let label = |s| {
            CompactMoveLabel::from_move(Move::from_usi(s).unwrap(), Color::BLACK)
                .unwrap()
                .raw() as usize
        };
        let a = label("7g7f");
        let b = label("2g2f");
        let drop = label("P*7f");
        for group in 0..388 {
            if group % 4 == 1 || group % 4 == 2 {
                assert_eq!(c.code(a, group), c.code(b, group));
            }
            if group % 4 == 0 {
                assert_eq!(c.code(a, group), c.code(drop, group));
            }
        }
        assert!(
            (3..388)
                .step_by(4)
                .any(|group| c.code(a, group) != c.code(b, group))
        );
    }
}
fn default_steps() -> usize {
    2400
}
fn default_time_bins() -> usize {
    1
}
fn is_zero(value: &usize) -> bool {
    *value == 0
}
fn is_false(value: &bool) -> bool {
    !*value
}
fn default_input_replicas() -> usize {
    1
}
fn is_one(value: &usize) -> bool {
    *value == 1
}
impl Circuit {
    pub fn source_neurons(&self) -> &[usize] {
        if self.sources.is_empty() {
            &self.kc
        } else {
            &self.sources
        }
    }
    pub fn run(
        &self,
        g: &Graph,
        input_pool: &[usize],
        p: &rsshogi::board::Position,
        seed: u64,
        dense: &[f32],
    ) -> Result<lif::ResultData, String> {
        let bits = if self.categorical_input {
            crate::encoding::encode_categorical(p).to_vec()
        } else {
            crate::encoding::encode(p).to_vec()
        };
        let dim = bits.len();
        if !self.kc_afferents.is_empty() {
            // Fixed artificial stimulus; overlapping feature assignments are summed as events.
            let mut events = vec![Vec::new(); self.steps];
            let stimulus_seed = lif::mix(seed);
            for (feature, &bit) in bits.iter().enumerate() {
                if bit == 0 {
                    continue;
                }
                let target = self.kc_afferents
                    [(lif::mix(29 ^ feature as u64) % self.kc_afferents.len() as u64) as usize];
                for (t, step) in events.iter_mut().enumerate() {
                    let draw = lif::mix(
                        stimulus_seed
                            ^ (t as u64).wrapping_mul(0xd1342543de82ef95)
                            ^ (feature as u64).wrapping_mul(0x9e3779b97f4a7c15),
                    );
                    if ((draw >> 11) as f64) * (1.0 / 9007199254740992.0) < 0.045 {
                        step.push(target);
                    }
                }
            }
            let cfg = lif::Simulation {
                steps: self.steps,
                direct: self.kc_afferents.clone(),
                active: vec![true; self.kc_afferents.len()],
                rate_hz: 450.0,
                seed: stimulus_seed,
                outputs: self.mbon.clone(),
                disconnected: false,
                events: Some(events),
                trace: false,
            };
            return lif::simulate_gains_window(
                g,
                &cfg,
                Some(dense),
                self.time_bins,
                self.observation_start,
            );
        }
        if self.kc_input && self.categorical_input && self.kc_replicas > 1 {
            let map = lif::input_map(&self.kc, 29, self.kc.len());
            let mut events = vec![Vec::new(); self.steps];
            let stimulus_seed = lif::mix(seed);
            for (feature, &bit) in bits.iter().enumerate() {
                if bit == 0 {
                    continue;
                }
                for replica in 0..self.kc_replicas {
                    let target = map[(replica * dim + feature) % map.len()];
                    let slot = feature * self.kc_replicas + replica;
                    for (t, step) in events.iter_mut().enumerate() {
                        let draw = lif::mix(
                            stimulus_seed
                                ^ (t as u64).wrapping_mul(0xd1342543de82ef95)
                                ^ (slot as u64).wrapping_mul(0x9e3779b97f4a7c15),
                        );
                        if ((draw >> 11) as f64) * (1.0 / 9007199254740992.0)
                            < 0.045 / self.kc_replicas as f64
                        {
                            step.push(target);
                        }
                    }
                }
            }
            let cfg = lif::Simulation {
                steps: self.steps,
                direct: self.kc.clone(),
                active: vec![true; self.kc.len()],
                rate_hz: 450.0 / self.kc_replicas as f64,
                seed: stimulus_seed,
                outputs: self.mbon.clone(),
                disconnected: false,
                events: Some(events),
                trace: false,
            };
            return lif::simulate_gains_window(
                g,
                &cfg,
                Some(dense),
                self.time_bins,
                self.observation_start,
            );
        }
        let input_pool = if self.kc_input {
            &self.kc[..]
        } else {
            input_pool
        };
        let replicas = if self.kc_input && self.categorical_input {
            1
        } else {
            2
        };
        if input_pool.len() < dim * replicas {
            return Err("Input pool too small for fixed encoding".into());
        }
        let map = lif::input_map(input_pool, 29, dim * replicas);
        let mut direct = Vec::with_capacity(dim * replicas);
        let mut active = Vec::with_capacity(dim * replicas);
        for (f, &bit) in bits.iter().enumerate() {
            for replica in 0..replicas {
                direct.push(map[replica * dim + f]);
                active.push(bit != 0);
            }
        }
        let cfg = lif::Simulation {
            steps: self.steps,
            direct,
            active,
            // Match expected initial-position stimulus count (40 versus 60 active bits).
            rate_hz: (if self.categorical_input { 225.0 } else { 150.0 }) * 2.0 / replicas as f64,
            seed: lif::mix(seed),
            outputs: self.mbon.clone(),
            disconnected: false,
            events: None,
            trace: false,
        };
        lif::simulate_gains_window(g, &cfg, Some(dense), self.time_bins, self.observation_start)
    }
    pub fn validate(&self, g: &Graph) -> Result<(), String> {
        let n = self.edges.len();
        if !["kc-mbon-v1", "kc-mbon-v2", "mbon-input-v1"].contains(&self.version.as_str())
            || (self.version == "mbon-input-v1" && self.sources.is_empty())
            || self.steps == 0
            || self.steps > 2400
            || ![1, 4].contains(&self.time_bins)
            || self.observation_start >= self.steps
            || ![1, 4, 8].contains(&self.kc_replicas)
            || (self.kc_replicas > 1
                && (!self.kc_input || !self.categorical_input || !self.kc_afferents.is_empty()))
            || (!self.kc_afferents.is_empty() && (self.kc_input || !self.categorical_input))
            || (self.steps - self.observation_start) % self.time_bins != 0
            || n == 0
            || self.pres.len() != n
            || self.groups.len() != n
            || self.gains.len() != n
            || self
                .kc
                .iter()
                .chain(&self.mbon)
                .chain(&self.sources)
                .chain(&self.kc_afferents)
                .any(|&i| i >= g.ids.len())
        {
            return Err("Invalid circuit".into());
        }
        let mut allowed = vec![false; g.ids.len()];
        for &source in self.source_neurons() {
            allowed[source] = true;
        }
        for i in 0..n {
            let (e, p, c) = (self.edges[i], self.pres[i], self.groups[i]);
            if p >= g.ids.len()
                || e >= g.targets.len()
                || c >= self.mbon.len()
                || !allowed[p]
                || e < g.offsets[p]
                || e >= g.offsets[p + 1]
                || g.targets[e] as usize != self.mbon[c]
                || g.weights[e] <= 0
                || !self.gains[i].is_finite()
                || !(0.05..=3.0).contains(&self.gains[i])
            {
                return Err("Invalid plastic edge".into());
            }
        }
        if let Some(u) = &self.upstream {
            if u.edges.is_empty()
                || u.pres.len() != u.edges.len()
                || u.groups.len() != u.edges.len()
                || u.gains.len() != u.edges.len()
            {
                return Err("Invalid upstream arrays".into());
            }
            for i in 0..u.edges.len() {
                let (e, p, k) = (u.edges[i], u.pres[i], u.groups[i]);
                if p >= g.ids.len()
                    || e >= g.targets.len()
                    || k >= self.kc.len()
                    || e < g.offsets[p]
                    || e >= g.offsets[p + 1]
                    || g.targets[e] as usize != self.kc[k]
                    || g.weights[e] <= 0
                    || !u.gains[i].is_finite()
                    || !(0.05..=3.0).contains(&u.gains[i])
                {
                    return Err("Invalid upstream edge".into());
                }
            }
        }
        Ok(())
    }
    pub fn dense(&self, g: &Graph) -> Vec<f32> {
        let mut v = vec![1.0; g.weights.len()];
        for (&e, &gain) in self.edges.iter().zip(&self.gains) {
            v[e] = gain;
        }
        if let Some(u) = &self.upstream {
            for (&e, &gain) in u.edges.iter().zip(&u.gains) {
                v[e] = gain;
            }
        }
        v
    }
    pub fn code(&self, label: usize, group: usize) -> f64 {
        let key = if self.structured_code {
            let expanded = rsshogi::labels::policy::CompactMoveLabel::from_raw(label as u16)
                .expect("valid policy label")
                .expand();
            let class = expanded.class().raw() as usize;
            match group % 4 {
                0 => expanded.to_sq().raw() as usize,
                1 => 81 + if class < 20 { class % 10 } else { class - 10 },
                2 => {
                    98 + if class < 10 {
                        0
                    } else if class < 20 {
                        1
                    } else {
                        2
                    }
                }
                _ => 101 + label,
            }
        } else {
            label
        };
        if lif::mix(self.code_seed ^ (key as u64).wrapping_mul(0xd1342543de82ef95) ^ group as u64)
            & 1
            == 0
        {
            1.0
        } else {
            -1.0
        }
    }
    /// Spike-weighted synaptic drive into MBONs, relative to the same connections at gain=1.
    /// This probes existing synapses, not a trained external linear head.
    pub fn activity(&self, g: &Graph, counts: &[u32]) -> Vec<f64> {
        assert_eq!(counts.len(), g.ids.len() * self.time_bins);
        let mut sums = vec![0.0; self.mbon.len() * self.time_bins];
        let mut norms = vec![0.0; sums.len()];
        for bin in 0..self.time_bins {
            for i in 0..self.edges.len() {
                let w = counts[bin * g.ids.len() + self.pres[i]] as f64
                    * g.weights[self.edges[i]] as f64;
                let c = bin * self.mbon.len() + self.groups[i];
                sums[c] += w * self.gains[i] as f64;
                norms[c] += w;
            }
        }
        sums.iter()
            .zip(norms)
            .map(|(&s, n)| if n > 0.0 { 1.0 - s / n } else { 0.0 })
            .collect()
    }
    pub fn logits(&self, g: &Graph, counts: &[u32]) -> Vec<f64> {
        let activity = self.activity(g, counts);
        (0..1496)
            .map(|l| {
                activity
                    .iter()
                    .enumerate()
                    .map(|(c, &v)| self.code(l, c) * v)
                    .sum::<f64>()
                    / (activity.len() as f64).sqrt()
            })
            .collect()
    }
    /// Projected margin update on existing synapses, with spikes held constant.
    /// Re-select the strongest legal competitor using the latest gains.
    pub fn teach_legal(
        &mut self,
        g: &Graph,
        counts: &[u32],
        truth: usize,
        legal: &[usize],
        margin: f64,
    ) -> usize {
        assert!(legal.contains(&truth));
        let scores = self.logits(g, counts);
        let competitor = legal
            .iter()
            .copied()
            .filter(|&l| l != truth)
            .max_by(|&x, &y| scores[x].total_cmp(&scores[y]).then(y.cmp(&x)));
        match competitor {
            Some(other) => self.teach(g, counts, truth, other, margin),
            None => {
                self.samples += 1;
                0
            }
        }
    }

    /// Projected margin update on existing synapses, with spikes held constant.
    /// Uses the derivative of this exact decoder; does not backpropagate through LIF.
    pub fn teach(
        &mut self,
        g: &Graph,
        counts: &[u32],
        truth: usize,
        chosen: usize,
        margin: f64,
    ) -> usize {
        self.samples += 1;
        if truth == chosen {
            return 0;
        }
        let scores = self.logits(g, counts);
        let loss = (margin - scores[truth] + scores[chosen]).max(0.0);
        if loss == 0.0 {
            return 0;
        }
        let mut norms = vec![0.0; self.mbon.len() * self.time_bins];
        for bin in 0..self.time_bins {
            for i in 0..self.edges.len() {
                norms[bin * self.mbon.len() + self.groups[i]] +=
                    counts[bin * g.ids.len() + self.pres[i]] as f64
                        * g.weights[self.edges[i]] as f64;
            }
        }
        let signal: Vec<_> = (0..norms.len())
            .map(|c| self.code(truth, c) - self.code(chosen, c))
            .collect();
        let gradients: Vec<_> = (0..self.edges.len())
            .map(|i| {
                (0..self.time_bins)
                    .map(|bin| {
                        let group = bin * self.mbon.len() + self.groups[i];
                        if norms[group] == 0.0 {
                            0.0
                        } else {
                            -(counts[bin * g.ids.len() + self.pres[i]] as f64
                                * g.weights[self.edges[i]] as f64
                                / norms[group])
                                * signal[group]
                                / (norms.len() as f64).sqrt()
                        }
                    })
                    .sum::<f64>()
            })
            .collect();
        let norm: f64 = gradients.iter().map(|x| x * x).sum();
        if norm == 0.0 {
            return 0;
        }
        let step = (loss / norm).min(100.0);
        let mut changed = 0;
        for i in 0..self.edges.len() {
            let new = (self.gains[i] as f64 + step * gradients[i]).clamp(0.05, 3.0) as f32;
            if new != self.gains[i] {
                changed += 1;
                self.gains[i] = new;
            }
        }
        if changed > 0 {
            self.updates += 1;
        }
        changed
    }
    pub fn info(&self) -> serde_json::Value {
        let upstream_edges = self.upstream.as_ref().map_or(0, |u| u.edges.len());
        let upstream_changed = self
            .upstream
            .as_ref()
            .map_or(0, |u| u.gains.iter().filter(|&&v| v != 1.0).count());
        let plastic_sources = self.upstream.as_ref().map_or_else(
            || self.source_neurons().len(),
            |u| {
                self.source_neurons()
                    .iter()
                    .chain(&u.pres)
                    .collect::<std::collections::HashSet<_>>()
                    .len()
            },
        );
        serde_json::json!({"method":self.version,"positions":self.samples,"updates":self.updates,"plastic_edges":self.edges.len()+upstream_edges,"changed_edges":self.gains.iter().filter(|&&v|v!=1.0).count()+upstream_changed,"upstream_edges":upstream_edges,"plastic_targets":if upstream_edges>0 {"KCとMBON"} else {"MBON"},"kc":self.kc.len(),"plastic_sources":plastic_sources,"mbon":self.mbon.len(),"duration_ms":self.steps as f64*0.1,"observation_start_ms":self.observation_start as f64*0.1,"time_bins":self.time_bins,"structured_code":self.structured_code,
            "input_dimensions":if self.categorical_input {crate::encoding::CATEGORICAL_DIM} else {crate::encoding::DIM},
            "input_replicas":if self.kc_input && self.categorical_input {self.kc_replicas} else if !self.kc_afferents.is_empty() {1} else {2},
            "input_target":if !self.kc_afferents.is_empty() {"KCの上流ニューロン"} else if self.kc_input {"KC"} else {"感覚ニューロン"}})
    }
}
