use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{self, Read},
    path::Path,
};

#[derive(Deserialize)]
pub struct Graph {
    pub ids: Vec<u64>,
    pub offsets: Vec<usize>,
    pub targets: Vec<u32>,
    /// Signed contact counts; contact_gain is applied by the simulator.
    pub weights: Vec<i32>,
}

impl Graph {
    pub fn validate(&self) -> Result<(), String> {
        let n = self.ids.len();
        if n == 0
            || self.offsets.len() != n + 1
            || self.offsets[0] != 0
            || self.offsets[n] != self.targets.len()
            || self.weights.len() != self.targets.len()
            || self.offsets.windows(2).any(|p| p[0] > p[1])
            || self.targets.iter().any(|&i| i as usize >= n)
        {
            return Err("Invalid CSR graph".into());
        }
        Ok(())
    }
    pub fn read(path: &Path) -> io::Result<Self> {
        let mut f = File::open(path)?;
        let mut header = [0u8; 24];
        f.read_exact(&mut header)?;
        if &header[..8] != b"FLYCSR01" {
            return Err(io::Error::other("Invalid graph magic"));
        }
        let n = u64::from_le_bytes(header[8..16].try_into().unwrap()) as usize;
        let m = u64::from_le_bytes(header[16..24].try_into().unwrap()) as usize;
        let expected = 24u64
            .checked_add(
                (n as u64)
                    .checked_mul(16)
                    .ok_or(io::Error::other("Graph size overflow"))?,
            )
            .and_then(|v| v.checked_add(8))
            .and_then(|v| v.checked_add((m as u64).checked_mul(8)?))
            .ok_or(io::Error::other("Graph size overflow"))?;
        if f.metadata()?.len() != expected {
            return Err(io::Error::other("Graph length mismatch"));
        }
        fn bytes(f: &mut File, count: usize) -> io::Result<Vec<u8>> {
            let mut b = vec![0; count];
            f.read_exact(&mut b)?;
            Ok(b)
        }
        let ids = bytes(&mut f, n * 8)?
            .chunks_exact(8)
            .map(|b| u64::from_le_bytes(b.try_into().unwrap()))
            .collect();
        let offsets = bytes(&mut f, (n + 1) * 8)?
            .chunks_exact(8)
            .map(|b| u64::from_le_bytes(b.try_into().unwrap()) as usize)
            .collect();
        let targets = bytes(&mut f, m * 4)?
            .chunks_exact(4)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
            .collect();
        let weights = bytes(&mut f, m * 4)?
            .chunks_exact(4)
            .map(|b| i32::from_le_bytes(b.try_into().unwrap()))
            .collect();
        let g = Self {
            ids,
            offsets,
            targets,
            weights,
        };
        g.validate().map_err(io::Error::other)?;
        Ok(g)
    }
}

/// SplitMix64 finalizer with its Weyl increment; all arithmetic is modulo 2^64.
pub fn mix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e3779b97f4a7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    x ^ (x >> 31)
}

pub fn input_map(pool: &[usize], seed: u64, count: usize) -> Vec<usize> {
    assert!(count <= pool.len());
    let mut map = pool.to_vec();
    for i in (1..map.len()).rev() {
        let j = (mix(seed ^ i as u64) % (i as u64 + 1)) as usize;
        map.swap(i, j);
    }
    map.truncate(count);
    map
}

#[derive(Deserialize)]
pub struct Simulation {
    pub steps: usize,
    pub direct: Vec<usize>,
    pub active: Vec<bool>,
    pub rate_hz: f64,
    pub seed: u64,
    pub outputs: Vec<usize>,
    pub disconnected: bool,
    #[serde(default)]
    pub events: Option<Vec<Vec<usize>>>,
    #[serde(default)]
    pub trace: bool,
}

#[derive(Serialize)]
pub struct ResultData {
    /// Presynaptic eligibility at the end of presentation, tau=40 ms.
    /// Not serialized or used by the inference decoder.
    #[serde(skip)]
    pub eligibility: Vec<f64>,
    /// Integral of KC spikes against a DA trace from a pulse 20 ms before onset.
    #[serde(skip)]
    pub preceding_dopamine_overlap: Vec<f64>,
    /// Same traces separated by observation bin; inference never reads these.
    #[serde(skip)]
    pub temporal_eligibility: Vec<f64>,
    #[serde(skip)]
    pub temporal_preceding_overlap: Vec<f64>,
    pub counts: Vec<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub temporal_counts: Vec<u32>,
    pub output_mean_v: Vec<f64>,
    pub output_z: Vec<f64>,
    pub stimulus_events: usize,
    pub trace_v: Vec<Vec<f64>>,
    pub trace_g: Vec<Vec<f64>>,
    pub spikes: Vec<(usize, usize)>,
}

impl ResultData {
    pub fn policy_counts(&self) -> &[u32] {
        if self.temporal_counts.is_empty() {
            &self.counts
        } else {
            &self.temporal_counts
        }
    }
}

// Four independent membrane updates at a time; deliberately no fused multiply-add
// or reduced precision, so the scalar arithmetic and spike order are preserved.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn update_avx2(
    v: &mut [f64],
    g: &mut [f64],
    ready: &[usize],
    t: usize,
    am: f64,
    ag: f64,
    coupling: f64,
    fired: &mut Vec<usize>,
) {
    use std::arch::x86_64::*;
    assert_eq!(v.len(), g.len());
    assert_eq!(v.len(), ready.len());
    // Every unaligned load/store is within the four-element chunk. The caller
    // checks AVX2 once; usize is 64-bit on this target and times fit in i64.
    unsafe {
        let rest = _mm256_set1_pd(-52.0);
        let threshold = _mm256_set1_pd(-45.0);
        let am4 = _mm256_set1_pd(am);
        let ag4 = _mm256_set1_pd(ag);
        let coupling4 = _mm256_set1_pd(coupling);
        let time4 = _mm256_set1_epi64x(t as i64);
        let end = v.len() / 4 * 4;
        for i in (0..end).step_by(4) {
            let old_v = _mm256_loadu_pd(v.as_ptr().add(i));
            let old_g = _mm256_loadu_pd(g.as_ptr().add(i));
            let blocked = _mm256_castsi256_pd(_mm256_cmpgt_epi64(
                _mm256_loadu_si256(ready.as_ptr().add(i).cast()),
                time4,
            ));
            let membrane = _mm256_add_pd(
                _mm256_add_pd(rest, _mm256_mul_pd(_mm256_sub_pd(old_v, rest), am4)),
                _mm256_mul_pd(old_g, coupling4),
            );
            _mm256_storeu_pd(
                v.as_mut_ptr().add(i),
                _mm256_blendv_pd(membrane, old_v, blocked),
            );
            _mm256_storeu_pd(
                g.as_mut_ptr().add(i),
                _mm256_blendv_pd(_mm256_mul_pd(old_g, ag4), old_g, blocked),
            );
            let mut mask = _mm256_movemask_pd(_mm256_andnot_pd(
                blocked,
                _mm256_cmp_pd::<_CMP_GT_OQ>(membrane, threshold),
            )) as u32;
            while mask != 0 {
                fired.push(i + mask.trailing_zeros() as usize);
                mask &= mask - 1;
            }
        }
        for i in end..v.len() {
            if t >= ready[i] {
                v[i] = -52.0 + (v[i] + 52.0) * am + g[i] * coupling;
                g[i] *= ag;
                if v[i] > -45.0 {
                    fired.push(i);
                }
            }
        }
    }
}

pub fn simulate(graph: &Graph, cfg: &Simulation) -> Result<ResultData, String> {
    simulate_gains(graph, cfg, None)
}

/// Gains modify transmission on existing edges without changing their signs or topology.
pub fn simulate_gains(
    graph: &Graph,
    cfg: &Simulation,
    gains: Option<&[f32]>,
) -> Result<ResultData, String> {
    simulate_gains_binned(graph, cfg, gains, 1)
}

pub fn simulate_gains_binned(
    graph: &Graph,
    cfg: &Simulation,
    gains: Option<&[f32]>,
    bins: usize,
) -> Result<ResultData, String> {
    simulate_gains_window(graph, cfg, gains, bins, 0)
}

/// Preserve the full simulation while observing spikes after a propagation interval.
pub fn simulate_gains_window(
    graph: &Graph,
    cfg: &Simulation,
    gains: Option<&[f32]>,
    bins: usize,
    observation_start: usize,
) -> Result<ResultData, String> {
    if bins == 0 || observation_start >= cfg.steps || (cfg.steps - observation_start) % bins != 0 {
        return Err("Invalid temporal bins".into());
    }
    if gains.is_some_and(|g| g.len() != graph.weights.len()) {
        return Err("Gain length mismatch".into());
    }
    graph.validate()?;
    let n = graph.ids.len();
    if cfg.steps == 0
        || !cfg.rate_hz.is_finite()
        || !(0.0..=10000.0).contains(&cfg.rate_hz)
        || cfg.direct.len() != cfg.active.len()
        || cfg.direct.iter().chain(&cfg.outputs).any(|&i| i >= n)
        || cfg
            .events
            .as_ref()
            .is_some_and(|e| e.len() != cfg.steps || e.iter().flatten().any(|&i| i >= n))
    {
        return Err("Invalid simulation configuration".into());
    }
    let mut is_direct = vec![false; n];
    for &i in &cfg.direct {
        if is_direct[i] {
            return Err("Duplicate direct input".into());
        }
        is_direct[i] = true;
    }
    if cfg
        .events
        .as_ref()
        .is_some_and(|e| e.iter().flatten().any(|&i| !is_direct[i]))
    {
        return Err("Event outside direct pool".into());
    }
    let mut v = vec![-52.0; n];
    let mut g = vec![0.0; n];
    let mut ready = vec![0; n];
    let mut queue = vec![Vec::<usize>::new(); 19];
    let mut counts = vec![0; n];
    let mut eligibility = vec![0.0; n];
    let mut preceding_dopamine_overlap = vec![0.0; n];
    let mut temporal_eligibility = vec![0.0; bins * n];
    let mut temporal_preceding_overlap = vec![0.0; bins * n];
    let preceding_decay: Vec<f64> = (0..cfg.steps)
        .map(|t| (-(20.0 + t as f64 * 0.1) / 40.0).exp())
        .collect();
    let eligibility_decay: Vec<f64> = (0..cfg.steps)
        .map(|t| (-((cfg.steps - 1 - t) as f64) * 0.1 / 40.0).exp())
        .collect();
    let mut temporal_counts = if bins > 1 || observation_start > 0 {
        vec![0; bins * n]
    } else {
        Vec::new()
    };
    let mut output_sum = vec![0.0; cfg.outputs.len()];
    let mut trace_v = Vec::new();
    let mut trace_g = Vec::new();
    let mut spikes = Vec::new();
    let mut stimulus_events = 0;
    let am = (-0.1f64 / 20.0).exp();
    let ag = (-0.1f64 / 5.0).exp();
    let coupling = (am - ag) / 3.0;
    let probability = cfg.rate_hz * 0.1 / 1000.0;
    #[cfg(target_arch = "x86_64")]
    let vectorized = std::is_x86_feature_detected!("avx2");
    #[cfg(not(target_arch = "x86_64"))]
    let vectorized = false;
    for t in 0..cfg.steps {
        let mut fired = Vec::new();
        if vectorized {
            #[cfg(target_arch = "x86_64")]
            unsafe {
                update_avx2(&mut v, &mut g, &ready, t, am, ag, coupling, &mut fired);
            }
        } else {
            for i in 0..n {
                if t >= ready[i] {
                    v[i] = -52.0 + (v[i] + 52.0) * am + g[i] * coupling;
                    g[i] *= ag;
                    if v[i] > -45.0 {
                        fired.push(i);
                    }
                }
            }
        }
        if !cfg.disconnected {
            for &pre in &queue[t % 19] {
                for edge in graph.offsets[pre]..graph.offsets[pre + 1] {
                    let post = graph.targets[edge] as usize;
                    if t >= ready[post] {
                        g[post] += 0.275
                            * graph.weights[edge] as f64
                            * gains.map_or(1.0, |g| g[edge] as f64);
                    }
                }
            }
        }
        queue[t % 19].clear();
        if let Some(events) = &cfg.events {
            for &i in &events[t] {
                v[i] += 68.75;
                stimulus_events += 1;
            }
        } else {
            for (slot, (&i, &active)) in cfg.direct.iter().zip(&cfg.active).enumerate() {
                let draw = mix(cfg.seed
                    ^ (t as u64).wrapping_mul(0xd1342543de82ef95)
                    ^ (slot as u64).wrapping_mul(0x9e3779b97f4a7c15));
                if active && ((draw >> 11) as f64) * (1.0 / 9007199254740992.0) < probability {
                    v[i] += 68.75;
                    stimulus_events += 1;
                }
            }
        }
        for &i in &fired {
            counts[i] += 1;
            eligibility[i] += eligibility_decay[t];
            preceding_dopamine_overlap[i] += preceding_decay[t];
            if t >= observation_start {
                let k =
                    ((t - observation_start) / ((cfg.steps - observation_start) / bins)) * n + i;
                temporal_eligibility[k] += eligibility_decay[t];
                temporal_preceding_overlap[k] += preceding_decay[t];
            }
            if (bins > 1 || observation_start > 0) && t >= observation_start {
                temporal_counts[((t - observation_start)
                    / ((cfg.steps - observation_start) / bins))
                    * n
                    + i] += 1;
            }
            v[i] = -52.0;
            g[i] = 0.0;
            ready[i] = t + if is_direct[i] { 0 } else { 22 };
            if cfg.trace {
                spikes.push((t, i));
            }
        }
        if !cfg.disconnected {
            queue[(t + 18) % 19].extend(fired);
        }
        for (j, &i) in cfg.outputs.iter().enumerate() {
            output_sum[j] += v[i];
        }
        if cfg.trace {
            trace_v.push(v.clone());
            trace_g.push(g.clone());
        }
    }
    if v.iter()
        .chain(&g)
        .chain(&output_sum)
        .any(|x| !x.is_finite())
    {
        return Err("Non-finite neural state".into());
    }
    let output_mean_v: Vec<_> = output_sum.iter().map(|s| s / cfg.steps as f64).collect();
    let output_z = cfg
        .outputs
        .iter()
        .zip(&output_mean_v)
        .map(|(&i, &m)| counts[i] as f64 + (m + 52.0) / 7.0)
        .collect();
    Ok(ResultData {
        eligibility,
        preceding_dopamine_overlap,
        temporal_eligibility,
        temporal_preceding_overlap,
        counts,
        temporal_counts,
        output_mean_v,
        output_z,
        stimulus_events,
        trace_v,
        trace_g,
        spikes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_arch = "x86_64")]
    #[test]
    fn avx2_preserves_scalar_updates_and_order() {
        if !std::is_x86_feature_detected!("avx2") {
            return;
        }
        let am = (-0.1f64 / 20.0).exp();
        let ag = (-0.1f64 / 5.0).exp();
        let coupling = (am - ag) / 3.0;
        let mut v: Vec<_> = (0..67).map(|i| -65.0 + i as f64 * 0.7).collect();
        let mut g: Vec<_> = (0..67).map(|i| (i as f64 - 33.0) * 9.0).collect();
        let ready: Vec<_> = (0..67).map(|i| i % 23).collect();
        let (mut expected_v, mut expected_g) = (v.clone(), g.clone());
        for t in 0..32 {
            let mut expected_spikes = Vec::new();
            for i in 0..v.len() {
                if t >= ready[i] {
                    expected_v[i] = -52.0 + (expected_v[i] + 52.0) * am + expected_g[i] * coupling;
                    expected_g[i] *= ag;
                    if expected_v[i] > -45.0 {
                        expected_spikes.push(i);
                    }
                }
            }
            let mut spikes = Vec::new();
            unsafe {
                update_avx2(&mut v, &mut g, &ready, t, am, ag, coupling, &mut spikes);
            }
            assert_eq!(v, expected_v);
            assert_eq!(g, expected_g);
            assert_eq!(spikes, expected_spikes);
        }
    }
    #[test]
    fn delay_and_cut_control() {
        let graph = Graph {
            ids: vec![1, 2],
            offsets: vec![0, 1, 1],
            targets: vec![1],
            weights: vec![100],
        };
        let mut events = vec![vec![]; 60];
        events[0] = vec![0];
        let mut cfg = Simulation {
            steps: 60,
            direct: vec![0],
            active: vec![true],
            rate_hz: 0.0,
            seed: 1,
            outputs: vec![1],
            disconnected: false,
            events: Some(events),
            trace: true,
        };
        let r = simulate(&graph, &cfg).unwrap();
        assert_eq!(r.spikes[0], (1, 0));
        for neuron in 0..graph.ids.len() {
            let expected: f64 = r
                .spikes
                .iter()
                .filter(|&&(_, i)| i == neuron)
                .map(|&(t, _)| (-((cfg.steps - 1 - t) as f64) * 0.1 / 40.0).exp())
                .sum();
            assert!((r.eligibility[neuron] - expected).abs() < 1e-12);
            let expected_early: f64 = r
                .spikes
                .iter()
                .filter(|&&(_, i)| i == neuron)
                .map(|&(t, _)| (-(20.0 + t as f64 * 0.1) / 40.0).exp())
                .sum();
            assert!((r.preceding_dopamine_overlap[neuron] - expected_early).abs() < 1e-12);
        }
        assert_eq!(r.trace_g[18][1], 0.0);
        assert!((r.trace_g[19][1] - 27.5).abs() < 1e-12);
        assert!(r.output_z[0] > 0.0);
        let binned = simulate_gains_binned(&graph, &cfg, None, 4).unwrap();
        assert_eq!(r.counts, binned.counts);
        assert_eq!(r.output_mean_v, binned.output_mean_v);
        assert_eq!(r.spikes, binned.spikes);
        let mut expected = vec![0; 8];
        for &(t, i) in &r.spikes {
            expected[(t / 15) * 2 + i] += 1;
        }
        assert_eq!(binned.temporal_counts, expected);
        for bin in 0..4 {
            for neuron in 0..2 {
                let times: Vec<_> = r
                    .spikes
                    .iter()
                    .filter(|&&(t, i)| t / 15 == bin && i == neuron)
                    .map(|&(t, _)| t)
                    .collect();
                let late: f64 = times
                    .iter()
                    .map(|&t| (-((cfg.steps - 1 - t) as f64) * 0.1 / 40.0).exp())
                    .sum();
                let early: f64 = times
                    .iter()
                    .map(|&t| (-(20.0 + t as f64 * 0.1) / 40.0).exp())
                    .sum();
                assert!((binned.temporal_eligibility[bin * 2 + neuron] - late).abs() < 1e-12);
                assert!(
                    (binned.temporal_preceding_overlap[bin * 2 + neuron] - early).abs() < 1e-12
                );
            }
        }
        let shifted = simulate_gains_window(&graph, &cfg, None, 4, 20).unwrap();
        assert_eq!(r.counts, shifted.counts);
        assert_eq!(r.spikes, shifted.spikes);
        assert_eq!(r.output_mean_v, shifted.output_mean_v);
        let mut expected = vec![0; 8];
        for &(t, i) in &r.spikes {
            if t >= 20 {
                expected[((t - 20) / 10) * 2 + i] += 1;
            }
        }
        assert_eq!(shifted.temporal_counts, expected);
        assert!(simulate_gains_window(&graph, &cfg, None, 4, 60).is_err());
        assert!(simulate_gains_window(&graph, &cfg, None, 4, 19).is_err());
        cfg.disconnected = true;
        let r = simulate(&graph, &cfg).unwrap();
        assert_eq!(r.output_z, [0.0]);
    }
}
