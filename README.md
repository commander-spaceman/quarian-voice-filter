# quarian-voice-filter

Reusable Rust workspace for Quarian-style voice filtering.

This repository contains a Rust implementation of a Quarian-style voice processing chain with a reusable library crate and a CLI for offline WAV processing.

Inspired by the pitch-shifting model and DSP chain of
[Librosa](https://librosa.org/).

> Currently available for **Windows**. macOS and Linux support is in progress.

## Demo

<p align="center">
  <video src="https://github.com/user-attachments/assets/faf7a57d-6896-4f27-a7b4-781aaa0d3552" controls width="720"></video>
</p>

_Shoutout to my gf for lending her voice to the video_

## Workspace

- `lib`: core library crate
- `cli`: CLI crate for file-based processing
- `gui`: GUI crate (egui + eframe) with presets and real-time parameter control
- `tests`: integration-style regression and behavior tests

## Features

- WAV decode, mono downmix, and float WAV encode
- Pitch shifting for offline voice processing
- High-pass, low-pass, and notch filtering
- Drive / saturation and dry-wet mixing
- CLI for batch-style file processing
- GUI with presets, dark theme, and background processing

## Requirements

- Rust 1.96.0 or newer
- Cargo

## Build

Build the CLI:

```powershell
cargo build -p quarian-voice-filter-cli --release
```

Build the GUI:

```powershell
cargo build -p quarian-voice-filter-gui --release
```

## Usage

### GUI

```powershell
cargo run -p quarian-voice-filter-gui --release
```

The GUI provides:

- **Presets**: Default, Subtle, Heavy, and Radio comm
- **Sliders** for all DSP parameters with real-time preview
- **Open WAV** / **Save WAV** with native file dialogs
- **Background processing** with spinner indicator

<p align="center">
  <img width="482" height="752" alt="Screenshot of the Quarian Voice Filter desktop app showing a loaded WAV file, preset controls, pitch, filter, mix and saturation sliders, and a saved output confirmation." src="https://github.com/user-attachments/assets/a88d7f98-1ab8-4d1e-af83-cc5533c5c95c">
</p>

### CLI

```powershell
cargo run -p quarian-voice-filter-cli -- --input ".\input.wav" --output ".\output.wav"
```

Optional parameters:

- `--pitch-semitones`: pitch shift amount in semitones. Default: `1.0`
- `--dry-gain`: level of the unprocessed signal in the final mix. Default: `0.25`
- `--wet-gain`: level of the processed signal in the final mix. Default: `0.15`
- `--hpf`: high-pass filter cutoff in Hz. Default: `200.0`
- `--lpf`: low-pass filter cutoff in Hz. Default: `7000.0`
- `--notch`: notch filter center frequency in Hz. Default: `1000.0`
- `--drive`: saturation amount applied to the processed signal. Default: `0.05`

## Development

Run the workspace tests:

```powershell
cargo test
```

Build all crates:

```powershell
cargo build --workspace --release
```

## License

MIT
