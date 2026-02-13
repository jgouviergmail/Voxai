# Expert Streaming STT Fix Plan (Verified)

## Problem Analysis

Whisper was designed for 30s batch audio. Feeding it isolated 1-2s segments causes:
1. **Hallucinations** — not enough context ("de la beste", "de la douleur douleur")
2. **Temperature cascade** — 6 fallback retries (0.0→0.2→0.4→0.6→0.8→1.0), very slow
3. **`[_NOT_]` token** — `no_timestamps=true` adds this token to the prompt, confusing the decoder on short audio
4. **Lost phrase endings** — silence detector strips ALL trailing silence; word endings clipped
5. **End-of-recording loss** — final phrase discarded if too short

## Root Cause

The segments are too short (1-2s) and lack audio context. Whisper needs:
- **Minimum 3s of audio** for reliable results (community consensus, whisper.cpp stream example)
- **Silence→Speech→Silence structure** (how Whisper was trained)
- **No retries** for consistent streaming latency

## Solution: Two-Pronged Fix

### A. Better Audio Segments (`silence.rs`)

Add **pre-roll** and **trailing silence padding** to give Whisper proper audio structure.

1. **Pre-roll buffer (500ms)**: Keep a `VecDeque<f32>` of recent silence during `Idle` state. When speech starts (Idle→InSpeech), prepend this silence to the speech buffer. Gives Whisper a clear "silence → speech onset" boundary, preventing word-onset clipping.

2. **Trailing silence pad (500ms)**: When emitting a segment after silence detection, include up to 500ms of the trailing silence (instead of stripping all silence). Gives Whisper a "speech → silence" boundary, preventing word-ending loss.

3. **Increase `MIN_SPEECH_DURATION_MS`**: 800ms → 2000ms. Ensures segments contain ≥2s of **actual speech** (excluding pre-roll). With pre-roll (500ms) + speech (2000ms) + trailing (500ms) = 3s minimum total — above Whisper's reliable threshold.

4. **Track `pre_roll_len`**: Record how many pre-roll samples were prepended to the speech buffer. The `min_speech_samples` check must exclude pre-roll:
   ```rust
   let actual_speech = speech_end.saturating_sub(self.pre_roll_len);
   if actual_speech >= min_speech_samples { /* emit */ }
   ```
   Without this, a segment with 500ms pre-roll + 1500ms speech = 2000ms would falsely pass the 2000ms threshold despite having only 1500ms of actual speech.

5. **Flush (end of recording)**: Append 500ms of synthetic silence (zeros) at the end. Keep `MIN_FLUSH_DURATION_MS = 300ms` (low threshold since user explicitly stopped). **Important**: the length check must exclude `pre_roll_len` and be computed BEFORE appending the synthetic pad, otherwise a 10ms noise burst with 500ms pre-roll + 500ms pad = 1010ms would falsely pass the 300ms threshold.

**Updated constants:**
```rust
const PRE_ROLL_MS: u32 = 500;
const TRAILING_PAD_MS: u32 = 500;
const MIN_SPEECH_DURATION_MS: u32 = 2000;  // was 800
// MIN_FLUSH_DURATION_MS stays 300
```

**New fields in `SilenceDetector`:**
```rust
pre_roll: VecDeque<f32>,       // ring buffer of recent silence
pre_roll_capacity: usize,      // max samples = PRE_ROLL_MS * samples_per_ms()
pre_roll_len: usize,           // how many pre-roll samples were prepended to current segment
```

**State machine changes:**
- `Idle`: accumulate chunks in `pre_roll` VecDeque (cap at `pre_roll_capacity`, drain excess from front). When RMS > threshold → drain pre-roll into `speech_buffer`, record `pre_roll_len`, append speech chunk, transition to InSpeech.
- `InSpeech` / `TrailingSilence`: unchanged logic, just uses new emission calculation.
- Emission: `speech_end = buffer_len - silence_samples` (speech + pre-roll portion). `actual_speech = speech_end - pre_roll_len`. If `actual_speech >= min_speech_samples`, emit `speech_buffer[..speech_end + pad_samples]` where `pad_samples = min(500ms, silence_samples)`.
- Reset to Idle: clear `speech_buffer`, reset `pre_roll_len = 0`, `pre_roll` is empty (was drained).
- Flush: compute `content_len = segment.len() - pre_roll_len` (excludes pre-roll), THEN append `TRAILING_PAD_MS` worth of zeros, check `content_len >= MIN_FLUSH_DURATION_MS`.

### B. Optimized Whisper Parameters (`whisper.rs`)

For streaming (short audio < 30s at 16kHz = 480,000 samples):

1. **Keep `no_timestamps=true`** — only reliable way to prevent "single timestamp ending - skip entire chunk". With 3s+ segments, the `[_NOT_]` token impact is minimized.

2. **Keep `single_segment=true`** — prevent multi-segment splitting on short audio.

3. **Add `temperature_inc = 0.0`** — **disables ALL fallback retries**. Rationale (verified via 3 iterations):
   - With properly-padded 3s+ segments, temperature 0.0 produces reliable results
   - Each fallback retry costs a FULL re-decode (same latency as the original attempt)
   - With `temperature_inc=0.2` (default), worst case = 6 retries = **7x latency** — kills real-time feel
   - Higher temperature = more random output, often *worse* than temperature 0.0
   - whisper.cpp's own stream example uses `temperature_inc=0.0` for streaming
   - API confirmed: `set_temperature_inc(f32)` in whisper-rs 0.15.1

4. **`audio_ctx` NOT included** — removed after verification:
   - Uncertain whether it speeds encoder or only decoder cross-attention
   - Truncation risk: segments > 15s would lose tail-end audio
   - Marginal benefit vs. `temperature_inc=0.0` which eliminates the main latency issue

### C. Streaming Loop (`streaming.rs`)

No structural changes needed — already correct:
- Errors caught per-segment with `if let Err(e)` (no loop abort)
- Diagnostic logging (heartbeat, segment detection, transcription, injection)
- Flush at end of recording handles final speech

## Files to Modify

| File | Changes |
|------|---------|
| `src-tauri/src/audio/silence.rs` | Add pre-roll VecDeque + trailing pad + `pre_roll_len` tracking + increase MIN_SPEECH to 2000ms + update tests |
| `src-tauri/src/stt/whisper.rs` | Add `params.set_temperature_inc(0.0)` inside `< 480_000` block |

## Detailed Implementation

### silence.rs changes

```rust
use std::collections::VecDeque;  // NEW import

const SPEECH_THRESHOLD: f32 = 0.015;
const SILENCE_DURATION_MS: u32 = 1000;
const MIN_SPEECH_DURATION_MS: u32 = 2000;     // was 800
const MIN_FLUSH_DURATION_MS: u32 = 300;
const PRE_ROLL_MS: u32 = 500;                 // NEW
const TRAILING_PAD_MS: u32 = 500;             // NEW

pub struct SilenceDetector {
    state: SilenceState,
    sample_rate: u32,
    channels: u16,
    speech_buffer: Vec<f32>,
    pre_roll: VecDeque<f32>,        // NEW
    pre_roll_capacity: usize,       // NEW
    pre_roll_len: usize,            // NEW — tracks pre-roll in current segment
}
```

**`new()`**: compute `pre_roll_capacity = PRE_ROLL_MS * sample_rate * channels / 1000`.

**`process()` — Idle branch**:
```rust
SilenceState::Idle => {
    if rms > SPEECH_THRESHOLD {
        // Prepend pre-roll silence for Whisper context
        self.pre_roll_len = self.pre_roll.len();
        self.speech_buffer.reserve(self.pre_roll.len() + chunk.len());
        self.speech_buffer.extend(self.pre_roll.drain(..));
        self.speech_buffer.extend_from_slice(chunk);
        self.state = SilenceState::InSpeech;
    } else {
        // Accumulate silence in pre-roll ring buffer
        self.pre_roll.extend(chunk.iter().copied());
        if self.pre_roll.len() > self.pre_roll_capacity {
            let excess = self.pre_roll.len() - self.pre_roll_capacity;
            self.pre_roll.drain(..excess);
        }
    }
    None
}
```

**`process()` — TrailingSilence emission**:
```rust
if *silence_samples >= silence_threshold_samples {
    let speech_end = self.speech_buffer.len() - *silence_samples;
    let actual_speech = speech_end.saturating_sub(self.pre_roll_len);
    if actual_speech >= min_speech_samples {
        // Include up to TRAILING_PAD_MS of silence
        let pad_samples = (TRAILING_PAD_MS as usize * self.samples_per_ms())
            .min(*silence_samples);
        let segment = self.speech_buffer[..speech_end + pad_samples].to_vec();
        self.speech_buffer.clear();
        self.state = SilenceState::Idle;
        self.pre_roll_len = 0;
        Some(segment)
    } else {
        self.speech_buffer.clear();
        self.state = SilenceState::Idle;
        self.pre_roll_len = 0;
        None
    }
}
```

**`flush()`**:
```rust
pub fn flush(&mut self) -> Option<Vec<f32>> {
    let min_flush_samples = MIN_FLUSH_DURATION_MS as usize * self.samples_per_ms();
    if !self.speech_buffer.is_empty()
        && matches!(self.state, SilenceState::InSpeech | SilenceState::TrailingSilence { .. })
    {
        let mut segment = std::mem::take(&mut self.speech_buffer);
        // Check content length BEFORE padding (excludes pre-roll silence)
        let content_len = segment.len().saturating_sub(self.pre_roll_len);
        // Append synthetic trailing silence for Whisper context
        let pad = TRAILING_PAD_MS as usize * self.samples_per_ms();
        segment.extend(std::iter::repeat(0.0f32).take(pad));
        self.state = SilenceState::Idle;
        self.pre_roll_len = 0;
        if content_len >= min_flush_samples {
            Some(segment)
        } else {
            None
        }
    } else {
        None
    }
}
```

### whisper.rs changes

Inside the `if samples.len() < 480_000` block, add one line:
```rust
if samples.len() < 480_000 {
    params.set_single_segment(true);
    params.set_no_timestamps(true);
    params.set_temperature_inc(0.0);  // NEW — disable fallback retries for streaming
}
```

### Test updates (silence.rs)

**Existing tests to update:**

1. `test_silence_detector_speech_then_silence` — increase speech from 20→25 chunks (2500ms > 2000ms threshold, provides margin). Assert emitted segment is non-empty (unchanged assertion, still valid).

2. `test_silence_detector_stereo_channels` — same: 20→25 chunks for stereo.

3. `test_silence_detector_flush_in_progress` — unchanged (10 chunks = 1000ms speech > 300ms flush threshold). Flushed segment will now include 500ms synthetic trailing silence (longer than before). Assert `flushed.unwrap().len() >= 4800` still passes (1000ms speech + 500ms pad = 24000 samples >> 4800).

4. `test_silence_detector_flush_short_speech_kept` — unchanged (400ms speech > 300ms flush threshold). Pre-roll empty (no preceding silence). `content_len = 400ms >= 300ms` → accepted. Segment now includes 500ms trailing pad.

5. `test_silence_detector_short_speech_ignored` — unchanged (200ms speech < 2000ms threshold → rejected as before).

6. `test_silence_detector_pure_silence` — unchanged (no speech → nothing emitted).

**New tests to add:**

7. `test_pre_roll_prepended` — feed 10 silence chunks (1000ms), then 25 speech chunks (2500ms), then 12 silence chunks. Verify emitted segment length > 25×1600 (contains pre-roll + trailing pad). Verify first samples in segment are near-zero (pre-roll silence).

8. `test_flush_excludes_preroll_from_check` — feed 5 silence chunks (500ms → fills pre-roll), then 1 speech chunk (100ms, too short for 300ms flush threshold when pre-roll excluded), then flush. Should return `None` (100ms content < 300ms threshold despite pre-roll inflating total to 600ms).

## Expected Improvements

| Issue | Before | After |
|-------|--------|-------|
| Segment duration | ~1-2s speech only | ~3-4s with padding |
| Audio structure | Speech only (no context) | Silence→Speech→Silence |
| Hallucinations | Frequent (short context + `[_NOT_]`) | Rare (3s+ context dilutes `[_NOT_]`) |
| Temperature retries | Up to 6 (0.0→1.0 in 0.2 steps) | **Zero** (temperature_inc=0.0) |
| Worst-case latency | 7x (6 retries + original) | 1x (no retries) |
| Word onset clipping | Yes (no leading silence) | No (500ms pre-roll silence) |
| Phrase ending loss | Yes (all silence stripped) | No (500ms trailing silence kept) |
| End-of-recording loss | Possible (800ms min) | Rare (300ms min + 500ms synthetic pad) |

## Review Checklist (5 iterations completed)

| Critère | Statut | Notes |
|---------|--------|-------|
| **Complétude** | ✓ | 2 fichiers, toutes constantes/champs/imports listés |
| **Nommage** | ✓ | UPPER_SNAKE constants, lower_snake fields, conforme codebase |
| **Imports** | ✓ | `use std::collections::VecDeque;` — suit pattern `use std::*` |
| **API publique** | ✓ | Aucun changement de signature → streaming.rs inchangé |
| **DRY** | ✓ | `samples_per_ms()` réutilisé, pas de duplication cross-fichier |
| **YAGNI** | ✓ | audio_ctx retiré, pas de sliding window, pas de feature superflue |
| **KISS** | ✓ | VecDeque simple, min() simple, temperature_inc=0.0 |
| **SRP/SoC** | ✓ | SilenceDetector garde sa responsabilité unique, pas de couplage |
| **Rust idiomatique** | ✓ | drain, extend, saturating_sub, mem::take, repeat().take() |
| **Pas de panic** | ✓ | Indexing prouvé safe (speech_end + pad_samples ≤ len) |
| **Gestion erreurs** | ✓ | Infaillible (Option), pas de unwrap/expect |
| **Mémoire** | ✓ | VecDeque borné ~192KB max, pas d'allocation non-bornée |
| **Thread safety** | ✓ | Single task, VecDeque<f32> Send+Sync |
| **Tests** | ✓ | 6 existants mis à jour + 2 nouveaux |
| **Bug trouvé et corrigé** | ✓ | flush() check pre-roll exclusion |

## Verification

1. `cargo check` — zero errors
2. `cargo test` — all tests pass (6 updated + 2 new)
3. Manual test with `RUST_LOG=info cargo tauri dev`:
   - Hold Shift+Space, speak 3-4 sentences with natural pauses
   - Verify: no "single timestamp ending" in console
   - Verify: no hallucinations in transcribed text
   - Verify: word onsets preserved ("Bonjour" not "onjour")
   - Verify: word endings preserved (no trailing "...")
   - Verify: release key → final phrase is transcribed
   - Verify: consistent latency (no long pauses — 0 retries)
