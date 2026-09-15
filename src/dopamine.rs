//! A deliberately minimal, nonnegative dopamine-gated KC synaptic depression rule.
//! The teacher interface is artificial; DAN->MBON contacts are a targeting proxy.
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Channel {
    pub neuron: usize,
    pub targets: Vec<(usize, f64)>,
}

/// Each synapse sees only its presynaptic trace and its local dopamine pulse.
pub fn depress(gain: f32, trace: f64, dopamine: f64, eta: f64) -> f32 {
    (gain as f64 - eta * trace * dopamine).clamp(0.05, 3.0) as f32
}

/// Opposing timing windows, both driven by nonnegative pulses.
/// This is a phenomenological model, not a fitted receptor-kinetics model.
pub fn paired(
    gain: f32,
    late_trace: f64,
    early_overlap: f64,
    late_pulse: f64,
    early_pulse: f64,
    eta: f64,
) -> f32 {
    (gain as f64 + eta * (early_overlap * early_pulse - late_trace * late_pulse)).clamp(0.05, 3.0)
        as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coincidence_is_required_and_gain_is_bounded() {
        assert_eq!(depress(1.0, 0.0, 1.0, 0.1), 1.0);
        assert_eq!(depress(1.0, 2.0, 0.0, 0.1), 1.0);
        assert_eq!(depress(1.0, 2.0, 1.0, 0.1), 0.8);
        assert_eq!(depress(1.0, 200.0, 1.0, 0.1), 0.05);
    }
    #[test]
    fn timing_reverses_direction_without_negative_dopamine() {
        assert_eq!(paired(1.0, 2.0, 1.0, 1.0, 0.0, 0.1), 0.8);
        assert_eq!(paired(1.0, 2.0, 1.0, 0.0, 1.0, 0.1), 1.1);
        assert_eq!(paired(1.0, 2.0, 1.0, 0.0, 0.0, 0.1), 1.0);
        assert_eq!(paired(1.0, 0.0, 0.0, 1.0, 1.0, 0.1), 1.0);
    }
}
