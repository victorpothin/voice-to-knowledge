use crate::error::AppError;
use std::path::Path;
use std::sync::Arc;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// Global Whisper context to avoid reloading the model
static mut WHISPER_CONTEXT: Option<Arc<WhisperContext>> = None;

/// Initialize the Whisper context with the model
fn get_or_create_context(model_path: &str) -> Result<Arc<WhisperContext>, AppError> {
    unsafe {
        if let Some(ctx) = &WHISPER_CONTEXT {
            return Ok(Arc::clone(ctx));
        }

        let ctx = WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
            .map_err(|e| AppError::Whisper(format!("Failed to load model: {}", e)))?;

        let ctx = Arc::new(ctx);
        WHISPER_CONTEXT = Some(Arc::clone(&ctx));
        Ok(ctx)
    }
}

/// Run whisper-rs on an audio file and return the transcribed text.
pub async fn transcrever(
    _whisper_bin: &str,
    model_path: &str,
    audio_path: &Path,
) -> Result<String, AppError> {
    let model_path = model_path.to_string();
    let audio_path = audio_path.to_path_buf();

    let result = tokio::task::spawn_blocking(move || {
        whisper_internal(&model_path, &audio_path)
    })
    .await
    .map_err(|e| AppError::Whisper(format!("Task join error: {}", e)))??;

    Ok(result)
}

fn whisper_internal(model_path: &str, audio_path: &Path) -> Result<String, AppError> {
    let ctx = get_or_create_context(model_path)?;

    // Read and decode audio file
    let audio_data = read_audio_file(&audio_path)?;

    // Create state
    let mut state = ctx
        .create_state()
        .map_err(|e| AppError::Whisper(format!("Failed to create state: {}", e)))?;

    // Create params
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_suppress_blank(true);
    params.set_suppress_non_speech_tokens(true);
    params.set_token_timestamps(false);
    params.set_split_on_word(true);
    params.set_max_initial_ts(0.0);
    params.set_language(Some("pt"));
    params.set_translate(false);
    params.set_n_threads(4);

    // Run inference
    state
        .full(params, &audio_data)
        .map_err(|e| AppError::Whisper(format!("Inference failed: {}", e)))?;

    // Collect segments
    let mut text = String::new();
    let num_segments = state
        .full_n_segments()
        .map_err(|e| AppError::Whisper(format!("Failed to get segments: {}", e)))?;

    for i in 0..num_segments {
        let segment = state
            .full_get_segment_text(i)
            .map_err(|e| AppError::Whisper(format!("Failed to get segment: {}", e)))?;
        text.push_str(&segment);
    }

    Ok(if text.trim().is_empty() {
        "(sem saída)".to_string()
    } else {
        text.trim().to_string()
    })
}

/// Read audio file and convert to mono 16kHz PCM
fn read_audio_file(path: &Path) -> Result<Vec<f32>, AppError> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "wav" => read_wav_file(path),
        _ => read_wav_file(path), // Try WAV for all formats
    }
}

fn read_wav_file(path: &Path) -> Result<Vec<f32>, AppError> {
    use std::io::BufReader;

    let file = std::fs::File::open(path)
        .map_err(|e| AppError::Whisper(format!("Failed to open audio file: {}", e)))?;
    let reader = BufReader::new(file);

    let mut wav_reader = hound::WavReader::new(reader)
        .map_err(|e| AppError::Whisper(format!("Failed to parse WAV: {}", e)))?;

    let spec = wav_reader.spec();
    let sample_rate = spec.sample_rate as f32;
    let channels = spec.channels as usize;

    // Read all samples
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => {
            let max_value = (1 << (spec.bits_per_sample - 1)) as f32;
            wav_reader
                .samples::<i32>()
                .filter_map(|s| s.ok())
                .map(|s| s as f32 / max_value)
                .collect()
        }
        hound::SampleFormat::Float => wav_reader
            .samples::<f32>()
            .filter_map(|s| s.ok())
            .collect(),
    };

    // Convert to mono if stereo
    let mono_samples = if channels == 2 {
        let mut mono = Vec::with_capacity(samples.len() / 2);
        for i in (0..samples.len()).step_by(2) {
            if i + 1 < samples.len() {
                mono.push((samples[i] + samples[i + 1]) / 2.0);
            } else {
                mono.push(samples[i]);
            }
        }
        mono
    } else {
        samples
    };

    // Resample to 16kHz if needed
    let target_sample_rate = 16000f32;
    let resampled = if (sample_rate - target_sample_rate).abs() > 0.1 {
        resample(&mono_samples, sample_rate, target_sample_rate)
    } else {
        mono_samples
    };

    Ok(resampled)
}

/// Simple linear resampler
fn resample(samples: &[f32], from_rate: f32, to_rate: f32) -> Vec<f32> {
    let ratio = from_rate / to_rate;
    let new_len = (samples.len() as f32 / ratio) as usize;
    let mut result = Vec::with_capacity(new_len);

    for i in 0..new_len {
        let src_idx = (i as f32 * ratio) as usize;
        if src_idx < samples.len() {
            result.push(samples[src_idx]);
        }
    }

    result
}
