use std::fs;
use std::path::PathBuf;

use clap::Parser;
use quarian_voice_filter::{process_wav_bytes, QuarianVoiceFilterParams};

#[derive(Debug, Parser)]
#[command(version, about = "Apply the Quarian voice filter to a WAV file")]
struct Cli {
    #[arg(short, long, value_name = "INPUT")]
    input: PathBuf,

    #[arg(short, long, value_name = "OUTPUT")]
    output: PathBuf,

    #[arg(long)]
    pitch_semitones: Option<f32>,

    #[arg(long)]
    dry_gain: Option<f32>,

    #[arg(long)]
    wet_gain: Option<f32>,

    #[arg(long)]
    hpf: Option<f32>,

    #[arg(long)]
    lpf: Option<f32>,

    #[arg(long)]
    notch: Option<f32>,

    #[arg(long)]
    drive: Option<f32>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let input = fs::read(&cli.input)?;
    let params = build_params(&cli);
    let output = process_wav_bytes(&input, &params)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;

    fs::write(&cli.output, output)?;
    Ok(())
}

fn build_params(cli: &Cli) -> QuarianVoiceFilterParams {
    let mut params = QuarianVoiceFilterParams::default();

    if let Some(value) = cli.pitch_semitones {
        params.pitch_semitones = value;
    }
    if let Some(value) = cli.dry_gain {
        params.dry_gain = value;
    }
    if let Some(value) = cli.wet_gain {
        params.wet_gain = value;
    }
    if let Some(value) = cli.hpf {
        params.hpf = value;
    }
    if let Some(value) = cli.lpf {
        params.lpf = value;
    }
    if let Some(value) = cli.notch {
        params.notch = value;
    }
    if let Some(value) = cli.drive {
        params.drive = value;
    }

    params
}

#[cfg(test)]
mod tests {
    use super::{build_params, Cli};
    use quarian_voice_filter::QuarianVoiceFilterParams;
    use std::path::PathBuf;

    #[test]
    fn cli_uses_nara_defaults() {
        let cli = Cli {
            input: PathBuf::from("in.wav"),
            output: PathBuf::from("out.wav"),
            pitch_semitones: None,
            dry_gain: None,
            wet_gain: None,
            hpf: None,
            lpf: None,
            notch: None,
            drive: None,
        };

        let params = build_params(&cli);

        assert_eq!(
            params,
            QuarianVoiceFilterParams {
                pitch_semitones: 1.0,
                dry_gain: 0.25,
                wet_gain: 0.15,
                hpf: 200.0,
                lpf: 7_000.0,
                notch: 1_000.0,
                drive: 0.05,
            }
        );
    }

    #[test]
    fn cli_flags_override_defaults() {
        let cli = Cli {
            input: PathBuf::from("in.wav"),
            output: PathBuf::from("out.wav"),
            pitch_semitones: Some(1.5),
            dry_gain: Some(0.2),
            wet_gain: Some(0.7),
            hpf: Some(250.0),
            lpf: Some(6_000.0),
            notch: Some(900.0),
            drive: Some(0.05),
        };

        let params = build_params(&cli);

        assert_eq!(
            params,
            QuarianVoiceFilterParams {
                pitch_semitones: 1.5,
                dry_gain: 0.2,
                wet_gain: 0.7,
                hpf: 250.0,
                lpf: 6_000.0,
                notch: 900.0,
                drive: 0.05,
            }
        );
    }
}
