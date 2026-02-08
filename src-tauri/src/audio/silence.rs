use std::collections::VecDeque;

const SPEECH_THRESHOLD: f32 = 0.015;
const SILENCE_DURATION_MS: u32 = 1000;
/// High-pass filter cutoff frequency (Hz). Removes low-frequency noise
/// (ventilator, vibrations, HVAC rumble) that triggers false speech detection.
const HP_CUTOFF_HZ: f32 = 100.0;
const MIN_SPEECH_DURATION_MS: u32 = 1500;
/// Lower minimum for flush (end of recording). The user explicitly stopped,
/// so even a short trailing phrase should be kept.
const MIN_FLUSH_DURATION_MS: u32 = 300;
/// Leading silence prepended to speech segments for Whisper context.
const PRE_ROLL_MS: u32 = 500;
/// Trailing silence included after speech for Whisper context.
const TRAILING_PAD_MS: u32 = 500;
/// Minimum consecutive above-threshold chunks before Idle→InSpeech.
/// Filters noise bursts < 30ms that confuse Whisper.
const SPEECH_ONSET_CHUNKS: u32 = 3;

enum SilenceState {
    Idle,
    InSpeech,
    TrailingSilence { silence_samples: usize },
}

pub struct SilenceDetector {
    state: SilenceState,
    sample_rate: u32,
    channels: u16,
    speech_buffer: Vec<f32>,
    pre_roll: VecDeque<f32>,
    pre_roll_capacity: usize,
    pre_roll_len: usize,
    /// Consecutive above-threshold chunks seen while Idle (debounce counter).
    speech_onset_count: u32,
    /// Chunks buffered during onset detection (before confirmed speech).
    onset_buffer: Vec<f32>,
    /// High-pass filter state (previous output).
    hp_y: f32,
    /// High-pass filter state (previous input sample).
    hp_prev_x: f32,
    /// High-pass filter coefficient α = RC/(RC+T).
    hp_alpha: f32,
}

impl SilenceDetector {
    pub fn new(sample_rate: u32, channels: u16) -> Self {
        let pre_roll_capacity =
            PRE_ROLL_MS as usize * sample_rate as usize * channels as usize / 1000;
        Self {
            state: SilenceState::Idle,
            sample_rate,
            channels,
            speech_buffer: Vec::new(),
            pre_roll: VecDeque::with_capacity(pre_roll_capacity),
            pre_roll_capacity,
            pre_roll_len: 0,
            speech_onset_count: 0,
            onset_buffer: Vec::new(),
            hp_y: 0.0,
            hp_prev_x: 0.0,
            hp_alpha: {
                let rc = 1.0 / (2.0 * std::f32::consts::PI * HP_CUTOFF_HZ);
                let dt = 1.0 / sample_rate as f32;
                rc / (rc + dt)
            },
        }
    }

    /// Samples per millisecond accounting for all channels (interleaved data).
    fn samples_per_ms(&self) -> usize {
        self.sample_rate as usize * self.channels as usize / 1000
    }

    /// Compute RMS after applying a first-order high-pass filter (100Hz).
    /// Filters out low-frequency noise (fans, HVAC, vibrations) while
    /// passing speech frequencies. The filter state persists across chunks
    /// for continuous filtering. The original audio is NOT modified.
    fn compute_rms_highpass(&mut self, chunk: &[f32]) -> f32 {
        if chunk.is_empty() {
            return 0.0;
        }
        let alpha = self.hp_alpha;
        let mut sum = 0.0f32;
        for &sample in chunk {
            // y[n] = α * (y[n-1] + x[n] - x[n-1])
            self.hp_y = alpha * (self.hp_y + sample - self.hp_prev_x);
            self.hp_prev_x = sample;
            sum += self.hp_y * self.hp_y;
        }
        (sum / chunk.len() as f32).sqrt()
    }

    /// Process a chunk of raw audio. Returns `Some(Vec<f32>)` when a complete
    /// speech segment is detected (silence after sufficient speech).
    pub fn process(&mut self, chunk: &[f32]) -> Option<Vec<f32>> {
        let rms = self.compute_rms_highpass(chunk);
        let spm = self.samples_per_ms();
        let silence_threshold_samples = SILENCE_DURATION_MS as usize * spm;
        let min_speech_samples = MIN_SPEECH_DURATION_MS as usize * spm;
        let trailing_pad_samples = TRAILING_PAD_MS as usize * spm;

        match &mut self.state {
            SilenceState::Idle => {
                if rms > SPEECH_THRESHOLD {
                    self.speech_onset_count += 1;
                    self.onset_buffer.extend_from_slice(chunk);
                    if self.speech_onset_count >= SPEECH_ONSET_CHUNKS {
                        // Confirmed speech onset — prepend pre-roll + onset buffer
                        self.pre_roll_len = self.pre_roll.len();
                        self.speech_buffer.reserve(
                            self.pre_roll.len() + self.onset_buffer.len(),
                        );
                        self.speech_buffer.extend(self.pre_roll.drain(..));
                        self.speech_buffer.extend(self.onset_buffer.drain(..));
                        self.speech_onset_count = 0;
                        self.state = SilenceState::InSpeech;
                    }
                } else {
                    // Below threshold — reset onset counter and discard onset buffer
                    if self.speech_onset_count > 0 {
                        log::debug!(
                            "Silence detector: noise burst rejected ({} chunks < {})",
                            self.speech_onset_count,
                            SPEECH_ONSET_CHUNKS,
                        );
                        self.speech_onset_count = 0;
                        self.onset_buffer.clear();
                    }
                    // Accumulate silence in pre-roll ring buffer
                    self.pre_roll.extend(chunk.iter().copied());
                    if self.pre_roll.len() > self.pre_roll_capacity {
                        let excess = self.pre_roll.len() - self.pre_roll_capacity;
                        self.pre_roll.drain(..excess);
                    }
                }
                None
            }
            SilenceState::InSpeech => {
                self.speech_buffer.extend_from_slice(chunk);
                if rms < SPEECH_THRESHOLD {
                    self.state = SilenceState::TrailingSilence {
                        silence_samples: chunk.len(),
                    };
                }
                None
            }
            SilenceState::TrailingSilence { silence_samples } => {
                self.speech_buffer.extend_from_slice(chunk);
                if rms > SPEECH_THRESHOLD {
                    self.state = SilenceState::InSpeech;
                    None
                } else {
                    *silence_samples += chunk.len();
                    if *silence_samples >= silence_threshold_samples {
                        let speech_end = self.speech_buffer.len() - *silence_samples;
                        let actual_speech = speech_end.saturating_sub(self.pre_roll_len);
                        if actual_speech >= min_speech_samples {
                            // Include up to TRAILING_PAD_MS of silence
                            let pad_samples = trailing_pad_samples.min(*silence_samples);
                            let segment =
                                self.speech_buffer[..speech_end + pad_samples].to_vec();
                            let spm = self.samples_per_ms();
                            log::info!(
                                "Silence detector: emitting segment — speech={}ms, pre_roll={}ms, pad={}ms, total={}ms",
                                actual_speech / spm,
                                self.pre_roll_len / spm,
                                pad_samples / spm,
                                segment.len() / spm,
                            );
                            self.speech_buffer.clear();
                            self.state = SilenceState::Idle;
                            self.pre_roll_len = 0;
                            Some(segment)
                        } else {
                            // Don't discard — keep buffer and accumulate with next phrase.
                            // Two short phrases (800ms each) with a 1s gap merge into
                            // one 2600ms segment that passes the threshold.
                            let spm = self.samples_per_ms();
                            log::debug!(
                                "Silence detector: short segment ({}ms < {}ms), keeping for accumulation",
                                actual_speech / spm,
                                MIN_SPEECH_DURATION_MS,
                            );
                            self.state = SilenceState::InSpeech;
                            None
                        }
                    } else {
                        None
                    }
                }
            }
        }
    }

    /// Returns the final segment if speech is still in progress without
    /// a trailing silence. Called at the end of recording.
    pub fn flush(&mut self) -> Option<Vec<f32>> {
        let min_flush_samples = MIN_FLUSH_DURATION_MS as usize * self.samples_per_ms();
        if !self.speech_buffer.is_empty()
            && matches!(
                self.state,
                SilenceState::InSpeech | SilenceState::TrailingSilence { .. }
            )
        {
            let mut segment = std::mem::take(&mut self.speech_buffer);
            // Check content length BEFORE padding (excludes pre-roll silence)
            let content_len = segment.len().saturating_sub(self.pre_roll_len);
            // Append synthetic trailing silence for Whisper context
            let pad = TRAILING_PAD_MS as usize * self.samples_per_ms();
            segment.extend(std::iter::repeat(0.0f32).take(pad));
            let spm = self.samples_per_ms();
            self.state = SilenceState::Idle;
            self.pre_roll_len = 0;
            if content_len >= min_flush_samples {
                log::info!(
                    "Silence detector: flush emitting segment — content={}ms, total={}ms",
                    content_len / spm,
                    segment.len() / spm,
                );
                Some(segment)
            } else {
                log::debug!(
                    "Silence detector: flush discarding short segment — content={}ms (min={}ms)",
                    content_len / spm,
                    MIN_FLUSH_DURATION_MS,
                );
                None
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
fn compute_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generate a test signal that passes the high-pass filter.
    /// Uses alternating ±amplitude (Nyquist/2 frequency), well above the 100Hz cutoff.
    /// RMS equals the amplitude value, same as a constant signal.
    fn speech(amplitude: f32, num_samples: usize) -> Vec<f32> {
        (0..num_samples)
            .map(|i| if i % 2 == 0 { amplitude } else { -amplitude })
            .collect()
    }

    #[test]
    fn test_compute_rms_empty() {
        assert_eq!(compute_rms(&[]), 0.0);
    }

    #[test]
    fn test_compute_rms_silence() {
        let silence = vec![0.0f32; 100];
        assert_eq!(compute_rms(&silence), 0.0);
    }

    #[test]
    fn test_compute_rms_signal() {
        let signal = vec![0.1f32; 100];
        let rms = compute_rms(&signal);
        assert!((rms - 0.1).abs() < 0.001);
        // Alternating signal should also have the same RMS
        let alt_signal = speech(0.1, 100);
        let alt_rms = compute_rms(&alt_signal);
        assert!((alt_rms - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_silence_detector_pure_silence() {
        let mut detector = SilenceDetector::new(16000, 1);
        let silence = vec![0.0f32; 1600]; // 100ms of silence
        for _ in 0..20 {
            assert!(detector.process(&silence).is_none());
        }
        assert!(detector.flush().is_none());
    }

    #[test]
    fn test_silence_detector_speech_then_silence() {
        let mut detector = SilenceDetector::new(16000, 1);
        // 2.5s of speech (above threshold) — needs to exceed MIN_SPEECH_DURATION_MS (1500ms)
        let speech = speech(0.05, 1600); // 100ms chunks
        for _ in 0..25 {
            assert!(detector.process(&speech).is_none());
        }
        // 1.2s of silence (above the 1000ms threshold)
        let silence = vec![0.0f32; 1600];
        let mut found = false;
        for _ in 0..12 {
            if let Some(segment) = detector.process(&silence) {
                assert!(!segment.is_empty());
                found = true;
                break;
            }
        }
        assert!(found, "Should have emitted a speech segment");
    }

    #[test]
    fn test_silence_detector_flush_in_progress() {
        let mut detector = SilenceDetector::new(16000, 1);
        // 1s of speech with no trailing silence (above MIN_FLUSH_DURATION_MS)
        let speech = speech(0.05, 1600);
        for _ in 0..10 {
            assert!(detector.process(&speech).is_none());
        }
        let flushed = detector.flush();
        assert!(flushed.is_some());
        // 1000ms speech + 500ms trailing pad = 24000 samples at 16kHz mono
        assert!(flushed.unwrap().len() >= 4800); // >= 300ms (MIN_FLUSH_DURATION_MS) at 16kHz mono
    }

    #[test]
    fn test_silence_detector_flush_short_speech_kept() {
        let mut detector = SilenceDetector::new(16000, 1);
        // 400ms of speech — below MIN_SPEECH_DURATION_MS (1500ms) but above
        // MIN_FLUSH_DURATION_MS (300ms), so flush should keep it.
        let speech = speech(0.05, 1600); // 100ms chunks
        for _ in 0..4 {
            assert!(detector.process(&speech).is_none());
        }
        let flushed = detector.flush();
        assert!(flushed.is_some(), "Flush should keep speech >= 300ms");
    }

    #[test]
    fn test_silence_detector_short_speech_ignored() {
        let mut detector = SilenceDetector::new(16000, 1);
        // 2 speech chunks (200ms) — below SPEECH_ONSET_CHUNKS (3), so debounce
        // rejects the burst and the state never leaves Idle.
        let speech = speech(0.05, 1600);
        detector.process(&speech);
        detector.process(&speech);
        // Then silence — onset counter resets, no segment emitted
        let silence = vec![0.0f32; 1600];
        for _ in 0..12 {
            assert!(detector.process(&silence).is_none());
        }
    }

    #[test]
    fn test_silence_detector_stereo_channels() {
        // 48kHz stereo: samples_per_ms = 48000 * 2 / 1000 = 96
        let mut detector = SilenceDetector::new(48000, 2);
        // 2.5s of speech in stereo
        let speech = speech(0.05, 9600); // 100ms stereo chunks (48000*2/10)
        for _ in 0..25 {
            assert!(detector.process(&speech).is_none());
        }
        // 1.2s of silence
        let silence = vec![0.0f32; 9600];
        let mut found = false;
        for _ in 0..12 {
            if let Some(segment) = detector.process(&silence) {
                assert!(!segment.is_empty());
                found = true;
                break;
            }
        }
        assert!(found, "Should have emitted a speech segment for stereo input");
    }

    #[test]
    fn test_pre_roll_prepended() {
        let mut detector = SilenceDetector::new(16000, 1);
        // 1s of silence to fill pre-roll buffer (capped at 500ms = 8000 samples)
        let silence = vec![0.0f32; 1600]; // 100ms chunks
        for _ in 0..10 {
            assert!(detector.process(&silence).is_none());
        }
        // 2.5s of speech
        let speech = speech(0.05, 1600);
        for _ in 0..25 {
            assert!(detector.process(&speech).is_none());
        }
        // 1.2s of silence to trigger emission
        let mut segment = None;
        for _ in 0..12 {
            if let Some(s) = detector.process(&silence) {
                segment = Some(s);
                break;
            }
        }
        let segment = segment.expect("Should have emitted a segment");
        // Segment should contain: pre-roll (500ms) + speech (2500ms) + trailing pad (500ms)
        // = 3500ms = 56000 samples at 16kHz mono
        // At minimum it must be larger than speech-only (25 * 1600 = 40000)
        assert!(
            segment.len() > 40000,
            "Segment should include pre-roll + trailing pad, got {} samples",
            segment.len()
        );
        // First samples should be near-zero (pre-roll silence)
        let first_rms = compute_rms(&segment[..1600]);
        assert!(
            first_rms < SPEECH_THRESHOLD,
            "First 100ms should be silence (pre-roll), RMS={}",
            first_rms
        );
    }

    #[test]
    fn test_flush_excludes_preroll_from_check() {
        let mut detector = SilenceDetector::new(16000, 1);
        // 500ms of silence to fill pre-roll buffer (8000 samples)
        let silence = vec![0.0f32; 1600]; // 100ms chunks
        for _ in 0..5 {
            assert!(detector.process(&silence).is_none());
        }
        // 3 × 80ms speech chunks (1280 samples each) — passes debounce (3 chunks)
        // but content = 240ms < 300ms flush threshold.
        // Without pre-roll exclusion, total = 500ms + 240ms = 740ms > 300ms → would incorrectly emit.
        let short_speech = speech(0.05, 1280); // 80ms per chunk
        for _ in 0..3 {
            assert!(detector.process(&short_speech).is_none());
        }
        // Flush: content_len = 240ms (speech only, pre-roll excluded) < 300ms → None
        let flushed = detector.flush();
        assert!(
            flushed.is_none(),
            "Flush should reject 240ms content even with 500ms pre-roll inflating total to 740ms"
        );
    }

    #[test]
    fn test_short_phrases_accumulated() {
        let mut detector = SilenceDetector::new(16000, 1);
        // Phrase 1: 800ms speech (below 1500ms threshold)
        let speech = speech(0.05, 1600); // 100ms chunks
        for _ in 0..8 {
            assert!(detector.process(&speech).is_none());
        }
        // 1.2s of silence — triggers threshold but speech too short → accumulated
        let silence = vec![0.0f32; 1600];
        for _ in 0..12 {
            assert!(detector.process(&silence).is_none());
        }
        // Phrase 2: 800ms speech — now total actual_speech includes both phrases + gap
        for _ in 0..8 {
            assert!(detector.process(&speech).is_none());
        }
        // 1.2s of silence — should now emit (800+1000+800 = 2600ms actual speech > 1500ms)
        let mut found = false;
        for _ in 0..12 {
            if let Some(segment) = detector.process(&silence) {
                assert!(!segment.is_empty());
                found = true;
                break;
            }
        }
        assert!(found, "Two short phrases should accumulate into one emitted segment");
    }

    #[test]
    fn test_noise_burst_rejected() {
        let mut detector = SilenceDetector::new(16000, 1);
        // 1-2 chunks of noise (< SPEECH_ONSET_CHUNKS=3) should NOT trigger speech
        let noise = speech(0.05, 1600); // 100ms
        let silence = vec![0.0f32; 1600];
        // Send 2 noise chunks (below debounce threshold of 3)
        assert!(detector.process(&noise).is_none());
        assert!(detector.process(&noise).is_none());
        // Then silence — resets onset counter, no transition
        for _ in 0..15 {
            assert!(detector.process(&silence).is_none());
        }
        // Flush should return None (never entered InSpeech)
        assert!(detector.flush().is_none());
    }

    #[test]
    fn test_debounce_allows_sustained_speech() {
        let mut detector = SilenceDetector::new(16000, 1);
        // 3+ consecutive above-threshold chunks should pass debounce
        let speech = speech(0.05, 1600); // 100ms chunks
        let silence = vec![0.0f32; 1600];
        // Send exactly SPEECH_ONSET_CHUNKS (3) speech chunks — triggers InSpeech
        // Then more speech to exceed MIN_SPEECH_DURATION_MS
        for _ in 0..20 {
            assert!(detector.process(&speech).is_none());
        }
        // 1.2s of silence to trigger emission
        let mut found = false;
        for _ in 0..12 {
            if let Some(segment) = detector.process(&silence) {
                assert!(!segment.is_empty());
                found = true;
                break;
            }
        }
        assert!(found, "Sustained speech should pass debounce and emit");
    }
}
