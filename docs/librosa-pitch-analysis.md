# Librosa Pitch Shift Analysis

This note captures the exact subset of Librosa behavior used by Nara's original
`scripts/quarian_fx.py` and the recommended Rust architecture to replace the
current simplified `lib/src/pitch.rs` implementation.

## What the Python code actually uses

In `scripts/quarian_fx.py`, Librosa is only used here:

```python
y_wet = librosa.effects.pitch_shift(y=y, sr=sr, n_steps=p["pitch_semitones"])
```

Everything else in the effect chain is already ported independently in Rust:

- mono downmix
- high-pass filter
- low-pass filter
- notch filter
- drive / tanh saturation
- dry/wet mix
- peak normalization

So the remaining fidelity gap is not "Librosa" in general. It is the specific
`pitch_shift` pipeline.

## Exact Librosa call chain

Reading the local installed source at
`C:/Users/juan/Documents/dev/nara/.venv/Lib/site-packages/librosa` shows:

### `librosa.effects.pitch_shift`

```python
rate = 2.0 ** (-float(n_steps) / bins_per_octave)

y_shift = core.resample(
    time_stretch(y, rate=rate, **kwargs),
    orig_sr=float(sr) / rate,
    target_sr=sr,
    res_type=res_type,
    scale=scale,
)

return util.fix_length(y_shift, size=y.shape[-1])
```

This means pitch shifting is implemented as:

1. time-stretch by `rate = 2^(-n_steps / bins_per_octave)`
2. resample back to the original sample rate
3. fix the output length to the original input length

### `librosa.effects.time_stretch`

```python
stft = core.stft(y, **kwargs)
stft_stretch = core.phase_vocoder(stft, rate=rate, ...)
len_stretch = int(round(y.shape[-1] / rate))
y_stretch = core.istft(stft_stretch, dtype=y.dtype, length=len_stretch, **kwargs)
```

So the minimum DSP path we need is:

1. STFT
2. phase vocoder
3. ISTFT
4. resample
5. fix length

## Minimum Librosa subset we actually need

We do **not** need a general-purpose port of Librosa. We only need a narrow,
voice-filter-focused subset:

### Required

- Hann window generation
- centered STFT framing
- real-input FFT to complex spectrum
- Librosa-style phase vocoder
- ISTFT overlap-add with window sum-square normalization
- final offline resample stage
- final fix-length crop/pad

### Not required for this feature

- multi-channel support inside pitch shifting
- alternate window types
- arbitrary padding modes
- generic spectrogram utilities
- amplitude/db helpers
- beat/onset/feature code
- general-purpose audio loading

## Key algorithm details from Librosa

### STFT defaults that matter

Librosa's `stft` defaults are important because `quarian_fx.py` does not pass
custom kwargs to `pitch_shift`:

- `n_fft = 2048`
- `win_length = n_fft`
- `hop_length = win_length // 4 = 512`
- `window = "hann"`
- `center = True`
- zero padding at both ends when centered

These defaults should be mirrored if we want comparable output.

### Phase vocoder behavior that matters

Librosa's `phase_vocoder` is a fairly small reference implementation. The core
behavior is:

- choose fractional time steps with `np.arange(0, frames, rate)`
- linearly interpolate magnitudes between adjacent frames
- accumulate phase using expected phase advance per bin
- wrap delta phase into `[-pi, pi]`

This is exactly the part our current `pitch.rs` does not reproduce.

### ISTFT behavior that matters

For fidelity, the important parts are:

- centered reconstruction logic
- overlap-add
- normalization by window sum-square
- explicit target length

### Resample behavior that matters

Librosa defaults to `res_type="soxr_hq"`, which is a high-quality band-limited
resampler. Our current pitch implementation does not do this. A closer Rust
version should use a higher-quality resampling stage than plain linear
interpolation.

## Recommended Rust architecture

The simplest maintainable architecture is to keep `lib/src/pitch.rs` as the
entry point, but move the DSP primitives into focused modules.

### Proposed modules

```text
lib/src/
  pitch.rs
  stft.rs
  phase_vocoder.rs
  resample.rs
```

### Responsibilities

- `pitch.rs`
  - public pitch-shift function
  - computes `rate`
  - orchestrates STFT -> phase vocoder -> ISTFT -> resample -> fix length

- `stft.rs`
  - Hann window creation
  - centered framing with zero padding
  - forward STFT
  - ISTFT overlap-add
  - window sum-square normalization

- `phase_vocoder.rs`
  - magnitude interpolation
  - phase advance computation
  - phase wrapping and accumulation

- `resample.rs`
  - offline mono resample helper
  - fixed-length output trim/pad

## Recommended Rust crates

### FFT

Use `rustfft`.

Why:

- pure Rust
- reusable planner API
- suitable for implementing STFT/ISTFT directly

Context7 confirms the intended use:

- `FftPlanner::<f32>::new()`
- `plan_fft_forward(len)`
- `plan_fft_inverse(len)`
- normalize manually after inverse FFT

### Resampling

Use `rubato` for the final resample stage.

Why:

- offline clip processing support
- higher-quality resampling than the current linear approximation
- `process_all_into_buffer` is a good fit for this use case

This will not exactly match `soxr_hq`, but it is much closer in spirit than the
current implementation.

## Recommended implementation scope for the next coding phase

### Phase A: internal DSP primitives

Implement:

- `stft.rs`
- `phase_vocoder.rs`
- `resample.rs`

Assumptions to hardcode initially:

- mono only
- `n_fft = 2048`
- `hop_length = 512`
- Hann window
- centered frames

This keeps the implementation small and aligned to the actual use case.

### Phase B: replace `pitch.rs`

Replace the current centered linear-resampling approximation with:

1. `rate = 2^(-n_steps / 12)`
2. `stft(samples)`
3. `phase_vocoder(stft, rate)`
4. `istft(..., length=round(len / rate))`
5. `resample(..., orig_sr = sr / rate, target_sr = sr)`
6. `fix_length(output, len_input)`

### Phase C: compare against Python baseline

Use the existing generated local fixtures to compare:

- `pitch_only_up_3`
- default profile

Metrics worth checking:

- exact output length
- RMS delta
- peak delta
- correlation against Python output
- spectral centroid shift

## Non-goals for now

These are out of scope unless audio quality still misses the target:

- transient preservation
- phase locking
- RubberBand-level quality
- sample-perfect parity with Librosa + soxr
- configurable STFT parameters in the public API

## Practical conclusion

The next fidelity step should **not** be "port more of Librosa". It should be:

1. implement a small STFT/phase-vocoder/ISTFT pipeline in Rust
2. use a proper offline resampler for the final stage
3. compare that output against the Python baseline fixtures

That reproduces the exact conceptual pipeline used by `librosa.effects.pitch_shift`
without importing the rest of Librosa's surface area.
