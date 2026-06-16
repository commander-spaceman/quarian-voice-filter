# Project Map

**Purpose:** Rust workspace for Quarian-style voice filtering — pitch shifting, filtering, saturation, and dry/wet mixing applied to mono WAV audio.

## Notes for AI Agents

- **Entry points:** `cli/src/main.rs` (CLI), `lib/src/lib.rs` (public API), `gui/src/main.rs` (GUI), `Cargo.toml` (workspace root)
- **Main patterns:** Pipeline architecture (pitch shift → filters → drive → mix → normalize). Each DSP stage is a separate module with a single public function. Pure functions, no async, no allocations in hot paths beyond vecs.
- **General rule:** Read this file before proposing structural changes or modifying multiple modules.

---

## 1. Workspace Root

Root Cargo workspace aggregating the library, CLI, and integration-test crates.

```text
.
├── Cargo.toml          # workspace definition, shared deps
├── Cargo.lock
├── .gitignore
├── LICENSE
├── README.md
├── assets/             # bundled static assets (fonts)
├── output/             # rendered audio outputs (gitignored)
├── lib/                # core library
├── cli/                # CLI binary
├── gui/                # GUI binary (egui/eframe)
└── tests/              # integration tests
```

**Main responsibilities:**

- Define workspace members and shared dependency versions
- Centralize package metadata (edition, license, rust-version) and shared dependencies (egui, eframe, rfd, chrono)

---

## 2. Core Library (`lib/`)

The reusable `quarian-voice-filter` crate. All DSP logic lives here.

```text
lib/
├── Cargo.toml
└── src/
    ├── lib.rs           # public API surface
    ├── params.rs        # configuration parameters
    ├── error.rs         # error types
    ├── dsp.rs           # main processing pipeline
    ├── filters.rs       # biquad IIR filters
    ├── pitch.rs         # pitch-shift orchestrator
    ├── stft.rs          # short-time Fourier transform
    ├── phase_vocoder.rs # phase vocoder time stretching
    ├── resample.rs      # sinc-based resampling
    └── wav.rs           # WAV decode/encode + mono downmix
```

**Main responsibilities:**

- Expose `process_wav_bytes()` and `process_mono_f32()` as the public API
- Define `QuarianVoiceFilterParams` (pitch, dry/wet gain, HPF, LPF, notch, drive)
- Implement the full voice-filtering DSP chain

**Key files:**

- `lib/src/lib.rs:15-68` — public entry points (`process_wav_bytes`, `process_wav_bytes_stereo`), validates inputs, delegates to `dsp`
- `lib/src/params.rs:5-13` — parameter struct with serde support and sensible defaults
- `lib/src/dsp.rs:8-38` — main pipeline orchestrating pitch → filters → drive → mix → normalize
- `lib/src/pitch.rs:5-25` — pitch shift via STFT → phase vocoder stretch → resample → fix length
- `lib/src/filters.rs:5-15` — 4th-order Butterworth HPF/LPF and notch filter
- `lib/src/wav.rs:8-20` — `OutputMode` enum (Preserve/ForceStereo), `MonoAudio` struct, decode/encode

**Relationships:**

- `dsp` depends on `pitch`, `filters`, and `params`
- `pitch` depends on `stft`, `phase_vocoder`, and `resample`
- `wav` is a `pub` module exposing `MonoAudio`, `OutputMode`, and `decode_wav_bytes`/`encode_wav_bytes`
- `error` is used by `lib` and `wav`

---

## 3. CLI (`cli/`)

Command-line binary for offline WAV file processing using `clap` for argument parsing.

```text
cli/
├── Cargo.toml
└── src/
    └── main.rs         # CLI entry point
```

**Main responsibilities:**

- Parse CLI arguments (`--input`, `--output`, `--pitch-semitones`, `--hpf`, `--lpf`, `--notch`, `--drive`, `--dry-gain`, `--wet-gain`)
- Read input WAV, apply filter, write output WAV

**Key files:**

- `cli/src/main.rs:7-75` — `Cli` struct, `main()` reads file, builds params, processes, writes result; `build_params()` helper; embedded `#[cfg(test)]` module

**Relationships:**

- Depends on `quarian-voice-filter` (the lib crate) via workspace dependency

---

## 4. Integration Tests (`tests/`)

Separate crate for integration-level regression and behavior tests.

```text
tests/
├── Cargo.toml
├── lib.rs              # crate root (minimal)
└── regression.rs       # integration tests
```

**Main responsibilities:**

- Test `process_wav_bytes` end-to-end with synthetic WAV data
- Test `process_mono_f32` with various parameter combinations
- Verify pitch shift increases estimated frequency, drive stays bounded, filters alter signal

**Key files:**

- `tests/regression.rs` — full integration tests exercising the public API

**Relationships:**

- Depends on `quarian-voice-filter` and `hound` for test WAV generation

---

## 5. GUI (`gui/`)

Desktop GUI application using egui/eframe for interactive voice filtering.

```text
gui/
├── Cargo.toml
└── src/
    └── main.rs         # eframe app, presets, parameter sliders, file I/O
```

**Main responsibilities:**

- Provide a graphical interface for loading WAV files, tweaking filter parameters with sliders, and saving processed output
- Offer built-in presets (Default, Subtle, Heavy, Radio comm)
- Handle drag-and-drop and file-picker (rfd) for input WAVs
- Run DSP processing on a background thread with `Arc<Mutex<>>` and poll for results
- Use `chrono` for timestamped default output filenames
- Embed `JetBrainsMono-Regular.ttf` from `assets/` as the UI font

**Key files:**

- `gui/src/main.rs:10-21` — `main()` sets up eframe with a 480×720 viewport
- `gui/src/main.rs:23-63` — `presets()` defines named parameter presets
- `gui/src/main.rs:89-119` — `App` struct holds all UI state
- `gui/src/main.rs:121-163` — `eframe::App` impl: dark theme, font setup, panel layout
- `gui/src/main.rs:430-457` — `start_process()` spawns a thread for `process_wav_bytes`
- `gui/src/main.rs:486-514` — `save_output()` opens rfd save dialog with chrono-generated filename

**Relationships:**

- Depends on `quarian-voice-filter` (the lib crate), `eframe`, `egui`, `rfd`, and `chrono` via workspace dependencies
- Embeds `assets/JetBrainsMono-Regular.ttf` for custom font rendering
