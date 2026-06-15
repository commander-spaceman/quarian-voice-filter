use std::io::{Cursor, Read, Seek};

use hound::{SampleFormat, WavReader, WavSpec, WavWriter};

use crate::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Preserve,
    ForceStereo,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MonoAudio {
    pub sample_rate: u32,
    pub samples: Vec<f32>,
    pub channels: u16,
}

pub fn decode_wav_bytes(input: &[u8]) -> Result<MonoAudio, Error> {
    if input.is_empty() {
        return Err(Error::InvalidInput("input WAV bytes cannot be empty"));
    }

    let normalized = normalize_wav_header(input);

    eprintln!(
        "[quarian-voice-filter] decode input bytes={} header={} chunks={}",
        normalized.len(),
        hex_prefix(&normalized, 32),
        riff_chunks(&normalized)
    );

    let cursor = Cursor::new(normalized);
    let mut reader = WavReader::new(cursor).map_err(|err| {
        eprintln!("[quarian-voice-filter] WavReader::new failed: {err}");
        Error::WavDecode(err)
    })?;
    let spec = reader.spec();

    eprintln!(
        "[quarian-voice-filter] decoded spec channels={} sample_rate={} bits={} format={:?}",
        spec.channels, spec.sample_rate, spec.bits_per_sample, spec.sample_format
    );

    if spec.channels == 0 {
        return Err(Error::InvalidInput("wav must have at least one channel"));
    }

    let interleaved = read_samples(&mut reader, spec)?;
    let samples = downmix_to_mono(&interleaved, spec.channels as usize);

    Ok(MonoAudio {
        sample_rate: spec.sample_rate,
        samples,
        channels: spec.channels,
    })
}

pub fn encode_wav_bytes(
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
    output_mode: OutputMode,
) -> Result<Vec<u8>, Error> {
    if sample_rate == 0 {
        return Err(Error::InvalidInput("sample_rate must be greater than zero"));
    }
    if channels == 0 {
        return Err(Error::InvalidInput("channels must be greater than zero"));
    }

    let output_channels = match output_mode {
        OutputMode::Preserve => channels,
        OutputMode::ForceStereo => channels.max(2),
    };

    let mut cursor = Cursor::new(Vec::new());
    let spec = WavSpec {
        channels: output_channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    eprintln!(
        "[quarian-voice-filter] encode spec channels={} sample_rate={} bits={} samples={}",
        spec.channels,
        spec.sample_rate,
        spec.bits_per_sample,
        samples.len()
    );

    {
        let mut writer = WavWriter::new(&mut cursor, spec).map_err(Error::WavEncode)?;
        for &sample in samples {
            let sample = f32_to_i16(sample);
            for _ in 0..output_channels {
                writer.write_sample(sample).map_err(Error::WavEncode)?;
            }
        }
        writer.finalize().map_err(Error::WavEncode)?;
    }

    Ok(cursor.into_inner())
}

fn hex_prefix(bytes: &[u8], count: usize) -> String {
    bytes
        .iter()
        .take(count)
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn riff_chunks(bytes: &[u8]) -> String {
    if bytes.len() < 12 {
        return "truncated".into();
    }

    let mut offset = 12usize;
    let mut chunks = Vec::new();

    while offset + 8 <= bytes.len() {
        let id = String::from_utf8_lossy(&bytes[offset..offset + 4]).into_owned();
        let size = u32::from_le_bytes([
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]) as usize;
        chunks.push(format!("{id}:{size}"));

        let padded = size + (size % 2);
        offset = offset.saturating_add(8).saturating_add(padded);
        if chunks.len() >= 8 {
            break;
        }
    }

    if chunks.is_empty() {
        "none".into()
    } else {
        chunks.join(", ")
    }
}

fn normalize_wav_header(input: &[u8]) -> Vec<u8> {
    let mut bytes = input.to_vec();

    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return bytes;
    }

    let riff_size = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    if riff_size == u32::MAX {
        let actual_riff_size = bytes.len().saturating_sub(8) as u32;
        bytes[4..8].copy_from_slice(&actual_riff_size.to_le_bytes());
    }

    let mut offset = 12usize;
    while offset + 8 <= bytes.len() {
        let id = &bytes[offset..offset + 4];
        let size = u32::from_le_bytes([
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]) as usize;

        if id == b"data" && size == u32::MAX as usize {
            let data_offset = offset + 8;
            let actual_data_size = bytes.len().saturating_sub(data_offset) as u32;
            bytes[offset + 4..offset + 8].copy_from_slice(&actual_data_size.to_le_bytes());
            break;
        }

        let padded = size + (size % 2);
        offset = offset.saturating_add(8).saturating_add(padded);
    }

    bytes
}

fn f32_to_i16(sample: f32) -> i16 {
    let clamped = sample.clamp(-1.0, 1.0);
    if clamped <= -1.0 {
        i16::MIN
    } else {
        (clamped * i16::MAX as f32).round() as i16
    }
}

fn read_samples<R>(reader: &mut WavReader<R>, spec: WavSpec) -> Result<Vec<f32>, Error>
where
    R: Read + Seek,
{
    match spec.sample_format {
        SampleFormat::Float => reader
            .samples::<f32>()
            .collect::<Result<Vec<_>, _>>()
            .map_err(Error::WavDecode),
        SampleFormat::Int => match spec.bits_per_sample {
            8 => reader
                .samples::<i8>()
                .map(|sample| sample.map(|value| value as f32 / i8::MAX as f32))
                .collect::<Result<Vec<_>, _>>()
                .map_err(Error::WavDecode),
            16 => reader
                .samples::<i16>()
                .map(|sample| sample.map(|value| value as f32 / i16::MAX as f32))
                .collect::<Result<Vec<_>, _>>()
                .map_err(Error::WavDecode),
            24 | 32 => reader
                .samples::<i32>()
                .map(|sample| sample.map(|value| value as f32 / i32::MAX as f32))
                .collect::<Result<Vec<_>, _>>()
                .map_err(Error::WavDecode),
            _ => Err(Error::InvalidInput("unsupported PCM bit depth")),
        },
    }
}

fn downmix_to_mono(samples: &[f32], channels: usize) -> Vec<f32> {
    if channels == 1 {
        return samples.to_vec();
    }

    samples
        .chunks_exact(channels)
        .map(|frame| frame.iter().copied().sum::<f32>() / channels as f32)
        .collect()
}
