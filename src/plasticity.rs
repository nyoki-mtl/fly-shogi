//! Learning resides on existing synapses into MBONs. The policy code is fixed.
use crate::lif::{self, Graph};
use serde::{Deserialize, Serialize};
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
    #[serde(default)]
    pub structured_code: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
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
        if !["kc-mbon-v2", "mbon-input-v1"].contains(&self.version.as_str())
            || (self.version == "mbon-input-v1" && self.sources.is_empty())
            || self.steps == 0
            || self.steps > 2400
            || ![1, 4].contains(&self.time_bins)
            || self.observation_start >= self.steps
            || ![1, 4, 8].contains(&self.kc_replicas)
            || (self.kc_replicas > 1 && (!self.kc_input || !self.categorical_input))
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
        Ok(())
    }
    pub fn dense(&self, g: &Graph) -> Vec<f32> {
        let mut v = vec![1.0; g.weights.len()];
        for (&e, &gain) in self.edges.iter().zip(&self.gains) {
            v[e] = gain;
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
    pub fn info(&self) -> serde_json::Value {
        serde_json::json!({"method":self.version,"positions":self.samples,"updates":self.updates,"plastic_edges":self.edges.len(),"changed_edges":self.gains.iter().filter(|&&v|v!=1.0).count(),"plastic_targets":"MBON","kc":self.kc.len(),"plastic_sources":self.source_neurons().len(),"mbon":self.mbon.len(),"duration_ms":self.steps as f64*0.1,"observation_start_ms":self.observation_start as f64*0.1,"time_bins":self.time_bins,"structured_code":self.structured_code,
            "input_dimensions":if self.categorical_input {crate::encoding::CATEGORICAL_DIM} else {crate::encoding::DIM},
            "input_replicas":if self.kc_input && self.categorical_input {self.kc_replicas} else {2},
            "input_target":if self.kc_input {"KC"} else {"感覚ニューロン"}})
    }
}
