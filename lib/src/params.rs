use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct QuarianVoiceFilterParams {
    pub pitch_semitones: f32,
    pub dry_gain: f32,
    pub wet_gain: f32,
    pub hpf: f32,
    pub lpf: f32,
    pub notch: f32,
    pub drive: f32,
}

impl Default for QuarianVoiceFilterParams {
    fn default() -> Self {
        Self {
            pitch_semitones: 1.0,
            dry_gain: 0.25,
            wet_gain: 0.15,
            hpf: 200.0,
            lpf: 7000.0,
            notch: 1000.0,
            drive: 0.05,
        }
    }
}
