import hashlib
import importlib.util
import io
import json
from pathlib import Path

import numpy as np
import soundfile as sf


ROOT = Path(__file__).resolve().parents[1]
FIXTURES_DIR = ROOT / "tests" / "fixtures" / "python-baseline"
INPUTS_DIR = FIXTURES_DIR / "inputs"
OUTPUTS_DIR = FIXTURES_DIR / "outputs"
NARA_ROOT = ROOT.parent / "nara"
QUARIAN_FX_PATH = NARA_ROOT / "scripts" / "quarian_fx.py"
SAMPLE_RATE = 24_000
SECONDS = 1.5

PROFILES = {
    "default": {},
    "pitch_only_up_3": {
        "pitch_semitones": 3,
        "dry_gain": 0.0,
        "wet_gain": 1.0,
        "hpf": 0,
        "lpf": SAMPLE_RATE,
        "notch": 0,
        "drive": 0.0,
    },
    "filter_drive_only": {
        "pitch_semitones": 0,
        "dry_gain": 0.0,
        "wet_gain": 1.0,
        "hpf": 200,
        "lpf": 7000,
        "notch": 1000,
        "drive": 0.05,
    },
}


def load_quarian_fx_module():
    spec = importlib.util.spec_from_file_location("quarian_fx", QUARIAN_FX_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


def generate_inputs():
    time = np.arange(int(SAMPLE_RATE * SECONDS), dtype=np.float32) / SAMPLE_RATE

    mono_voice_like = (
        0.45 * np.sin(2 * np.pi * 220 * time)
        + 0.25 * np.sin(2 * np.pi * 440 * time)
        + 0.15 * np.sin(2 * np.pi * 660 * time)
    ) * (0.75 + 0.25 * np.sin(2 * np.pi * 3 * time))

    sweep = np.sin(2 * np.pi * (180 + 1600 * time / SECONDS) * time)
    sweep *= np.linspace(0.4, 0.9, time.size, dtype=np.float32)

    left = 0.5 * np.sin(2 * np.pi * 196 * time)
    right = 0.35 * np.sin(2 * np.pi * 294 * time) + 0.2 * np.sin(2 * np.pi * 588 * time)
    stereo_dual_tone = np.stack([left, right], axis=1)

    return {
        "mono_voice_like": mono_voice_like.astype(np.float32),
        "mono_sweep": sweep.astype(np.float32),
        "stereo_dual_tone": stereo_dual_tone.astype(np.float32),
    }


def wav_bytes_from_array(samples: np.ndarray) -> bytes:
    buffer = io.BytesIO()
    sf.write(buffer, samples, SAMPLE_RATE, format="WAV")
    return buffer.getvalue()


def read_wav_metrics(wav_bytes: bytes) -> dict:
    samples, sample_rate = sf.read(io.BytesIO(wav_bytes), dtype="float32")
    samples = np.asarray(samples)
    mono_samples = samples.mean(axis=1) if samples.ndim > 1 else samples

    return {
        "sample_rate": int(sample_rate),
        "channels": int(samples.shape[1]) if samples.ndim > 1 else 1,
        "frames": int(samples.shape[0]),
        "peak": float(np.max(np.abs(mono_samples))) if mono_samples.size else 0.0,
        "rms": float(np.sqrt(np.mean(np.square(mono_samples))))
        if mono_samples.size
        else 0.0,
        "sha256": hashlib.sha256(wav_bytes).hexdigest(),
    }


def write_bytes(path: Path, data: bytes):
    path.write_bytes(data)


def main():
    quarian_fx = load_quarian_fx_module()
    inputs = generate_inputs()
    manifest = {
        "source_script": str(QUARIAN_FX_PATH),
        "sample_rate": SAMPLE_RATE,
        "seconds": SECONDS,
        "profiles": PROFILES,
        "fixtures": [],
    }

    for input_name, input_samples in inputs.items():
        input_wav = wav_bytes_from_array(input_samples)
        input_path = INPUTS_DIR / f"{input_name}.wav"
        write_bytes(input_path, input_wav)
        input_metrics = read_wav_metrics(input_wav)

        fixture_entry = {
            "input": {
                "name": input_name,
                "path": str(input_path.relative_to(ROOT)).replace("\\", "/"),
                "metrics": input_metrics,
            },
            "outputs": [],
        }

        for profile_name, params in PROFILES.items():
            output_wav = quarian_fx.apply(input_wav, params)
            output_path = OUTPUTS_DIR / f"{input_name}__{profile_name}.wav"
            write_bytes(output_path, output_wav)
            fixture_entry["outputs"].append(
                {
                    "profile": profile_name,
                    "params": params,
                    "path": str(output_path.relative_to(ROOT)).replace("\\", "/"),
                    "metrics": read_wav_metrics(output_wav),
                }
            )

        manifest["fixtures"].append(fixture_entry)

    manifest_path = FIXTURES_DIR / "manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
