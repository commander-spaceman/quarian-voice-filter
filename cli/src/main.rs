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
