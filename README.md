# quarian-voice-filter

Reusable Rust workspace for Quarian-style voice filtering.

This repository contains a Rust implementation of a Quarian-style voice
processing chain with a reusable library crate and a CLI for offline WAV
processing.

## Workspace

- `lib`: core library crate
- `cli`: CLI crate for file-based processing
- `tests`: integration-style regression and behavior tests

## Features

- WAV decode, mono downmix, and float WAV encode
- Pitch shifting for offline voice processing
- High-pass, low-pass, and notch filtering
- Drive / saturation and dry-wet mixing
- CLI for batch-style file processing

## Usage

Run the CLI on a WAV file:

```powershell
cargo run -p quarian-voice-filter-cli -- --input ".\input.wav" --output ".\output.wav"
```

Optional parameters:

- `--pitch-semitones`
- `--dry-gain`
- `--wet-gain`
- `--hpf`
- `--lpf`
- `--notch`
- `--drive`

## Development

Run the workspace tests:

```powershell
cargo test
```
