use std::io::{Cursor, Read, Seek};

use hound::{SampleFormat, WavReader, WavSpec, WavWriter};

use crate::Error;

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

    let cursor = Cursor::new(input);
    let mut reader = WavReader::new(cursor).map_err(Error::WavDecode)?;
    let spec = reader.spec();

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
) -> Result<Vec<u8>, Error> {
    if sample_rate == 0 {
        return Err(Error::InvalidInput("sample_rate must be greater than zero"));
    }
    if channels == 0 {
        return Err(Error::InvalidInput("channels must be greater than zero"));
    }

    let mut cursor = Cursor::new(Vec::new());
    let spec = WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };

    {
        let mut writer = WavWriter::new(&mut cursor, spec).map_err(Error::WavEncode)?;
        for &sample in samples {
            for _ in 0..channels {
                writer.write_sample(sample).map_err(Error::WavEncode)?;
            }
        }
        writer.finalize().map_err(Error::WavEncode)?;
    }

    Ok(cursor.into_inner())
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
