use rubato::{FftFixedIn, Resampler};

use crate::error::AppError;

const TARGET_SAMPLE_RATE: usize = 16_000;

/// Converts interleaved multi-channel audio to 16kHz mono f32 samples.
/// Input: interleaved samples at `source_rate` with `channels` channels.
/// Output: mono 16kHz f32 samples ready for Whisper.
pub fn resample_to_16k_mono(
    samples: &[f32],
    channels: u16,
    source_rate: u32,
) -> Result<Vec<f32>, AppError> {
    let channels = channels as usize;
    let source_rate = source_rate as usize;

    if channels == 0 || source_rate == 0 || samples.is_empty() {
        return Err(AppError::Audio(format!(
            "Invalid audio params: {} channels, {}Hz, {} samples",
            channels, source_rate, samples.len()
        )));
    }

    // Step 1: Deinterleave and downmix to mono
    let mono: Vec<f32> = if channels == 1 {
        samples.to_vec()
    } else {
        // Average all channels
        let frame_count = samples.len() / channels;
        let mut mono = Vec::with_capacity(frame_count);
        for frame in 0..frame_count {
            let mut sum = 0.0f32;
            for ch in 0..channels {
                sum += samples[frame * channels + ch];
            }
            mono.push(sum / channels as f32);
        }
        mono
    };

    // Step 2: Resample if needed
    if source_rate == TARGET_SAMPLE_RATE {
        return Ok(mono);
    }

    let chunk_size = 1024;
    let mut resampler = FftFixedIn::<f32>::new(
        source_rate,
        TARGET_SAMPLE_RATE,
        chunk_size,
        2, // sub_chunks
        1, // mono
    )
    .map_err(|e| AppError::Audio(format!("Failed to create resampler: {}", e)))?;

    // Pre-allocate output based on expected resampled length (avoids ~8-9 reallocations)
    let expected_len = mono.len() * TARGET_SAMPLE_RATE / source_rate + chunk_size;
    let mut output = Vec::with_capacity(expected_len);
    let mut pos = 0;

    while pos + chunk_size <= mono.len() {
        let chunk = &mono[pos..pos + chunk_size];
        let input = vec![chunk.to_vec()]; // Non-interleaved: Vec<Vec<f32>>
        let result = resampler
            .process(&input, None)
            .map_err(|e| AppError::Audio(format!("Resample error: {}", e)))?;
        if let Some(channel) = result.first() {
            output.extend_from_slice(channel);
        }
        pos += chunk_size;
    }

    // Handle remaining samples
    let remaining = mono.len() - pos;
    if remaining > 0 {
        let mut last_chunk = mono[pos..].to_vec();
        last_chunk.resize(chunk_size, 0.0); // Pad with silence
        let input = vec![last_chunk];
        let result = resampler
            .process(&input, None)
            .map_err(|e| AppError::Audio(format!("Resample error: {}", e)))?;
        if let Some(channel) = result.first() {
            // Only take proportional amount of output
            let expected = (remaining as f64 * TARGET_SAMPLE_RATE as f64 / source_rate as f64)
                .ceil() as usize;
            let take = expected.min(channel.len());
            output.extend_from_slice(&channel[..take]);
        }
    }

    log::info!(
        "Resampled: {}Hz {}ch -> 16kHz mono ({} -> {} samples)",
        source_rate,
        channels,
        samples.len(),
        output.len()
    );

    Ok(output)
}
