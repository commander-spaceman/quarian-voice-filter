use quarian_voice_filter::{process_mono_f32, process_wav_bytes, QuarianVoiceFilterParams};

#[test]
fn placeholders_return_not_implemented_errors() {
    let params = QuarianVoiceFilterParams::default();

    let wav_error = process_wav_bytes(&[], &params).unwrap_err();
    assert_eq!(
        wav_error.to_string(),
        "process_wav_bytes is not implemented yet"
    );

    let pcm_error = process_mono_f32(&[], 24_000, &params).unwrap_err();
    assert_eq!(
        pcm_error.to_string(),
        "process_mono_f32 is not implemented yet"
    );
}
