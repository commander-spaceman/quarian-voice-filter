# Project Map

**Purpose:** Rust workspace for Quarian-style voice filtering — pitch shifting, filtering, saturation, and dry/wet mixing applied to mono WAV audio.

## Notes for AI Agents

- **Entry points:** `cli/src/main.rs` (CLI), `lib/src/lib.rs` (public API), `Cargo.toml` (workspace root)
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
├── output/             # rendered audio outputs (gitignored)
├── lib/                # core library
├── cli/                # CLI binary
└── tests/              # integration tests
```

**Main responsibilities:**

- Define workspace members and shared dependency versions
- Centralize package metadata (edition, license, rust-version)

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

- `lib/src/lib.rs:15-34` — public entry points, validates inputs, delegates to `dsp`
- `lib/src/params.rs:5-13` — parameter struct with serde support and sensible defaults
- `lib/src/dsp.rs:8-38` — main pipeline orchestrating pitch → filters → drive → mix → normalize
- `lib/src/pitch.rs:5-25` — pitch shift via STFT → phase vocoder stretch → resample → fix length
- `lib/src/filters.rs:5-15` — 4th-order Butterworth HPF/LPF and notch filter
- `lib/src/wav.rs:14-68` — WAV bytes → mono f32 decode and f32 → WAV bytes encode

**Relationships:**

- `dsp` depends on `pitch`, `filters`, and `params`
- `pitch` depends on `stft`, `phase_vocoder`, and `resample`
- `wav` and `error` are used by `lib` to expose the public API

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

- `cli/src/main.rs:38-47` — `main()` reads file, builds params, processes, writes result

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
