use quarian_voice_filter::{process_wav_bytes, QuarianVoiceFilterParams};

fn main() {
    let params = QuarianVoiceFilterParams::default();
    let _ = process_wav_bytes(&[], &params);

    eprintln!(
        "quarian-voice-filter-cli scaffold is ready; file processing will land in a later phase"
    );
}
