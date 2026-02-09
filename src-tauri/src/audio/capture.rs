use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};

use crate::error::AppError;

struct ResolvedDevice {
    device: Device,
    config: StreamConfig,
    sample_format: SampleFormat,
}

fn resolve_input_device(device_name: Option<&str>) -> Result<ResolvedDevice, AppError> {
    let host = cpal::default_host();

    let device = match device_name {
        Some(name) => host
            .input_devices()?
            .find(|d| d.description().map(|desc| desc.name() == name).unwrap_or(false))
            .ok_or_else(|| AppError::Audio(format!("Device '{}' not found", name)))?,
        None => host
            .default_input_device()
            .ok_or_else(|| AppError::Audio("No default input device".to_string()))?,
    };

    let supported_config = device.default_input_config()?;
    let sample_format = supported_config.sample_format();
    let config: StreamConfig = supported_config.into();

    Ok(ResolvedDevice {
        device,
        config,
        sample_format,
    })
}

/// Build a cpal input stream that converts samples to f32 and calls `on_data`.
fn build_stream(
    resolved: &ResolvedDevice,
    on_data: impl Fn(&[f32]) + Send + 'static,
) -> Result<Stream, AppError> {
    let err_fn = |err: cpal::StreamError| {
        log::error!("Audio stream error: {}", err);
    };

    let stream = match resolved.sample_format {
        SampleFormat::F32 => {
            resolved.device.build_input_stream(
                &resolved.config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    on_data(data);
                },
                err_fn,
                None,
            )?
        }
        SampleFormat::I16 => {
            resolved.device.build_input_stream(
                &resolved.config,
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    let floats: Vec<f32> =
                        data.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
                    on_data(&floats);
                },
                err_fn,
                None,
            )?
        }
        SampleFormat::I32 => {
            resolved.device.build_input_stream(
                &resolved.config,
                move |data: &[i32], _: &cpal::InputCallbackInfo| {
                    let floats: Vec<f32> =
                        data.iter().map(|&s| s as f32 / i32::MAX as f32).collect();
                    on_data(&floats);
                },
                err_fn,
                None,
            )?
        }
        _ => {
            return Err(AppError::Audio(format!(
                "Unsupported sample format: {:?}",
                resolved.sample_format
            )));
        }
    };

    Ok(stream)
}

pub struct AudioCapture {
    stream: Option<Stream>,
    buffer: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    channels: u16,
}

impl AudioCapture {
    pub fn new() -> Self {
        Self {
            stream: None,
            buffer: Arc::new(Mutex::new(Vec::new())),
            sample_rate: 0,
            channels: 0,
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn start(&mut self, device_name: Option<&str>) -> Result<(), AppError> {
        let resolved = resolve_input_device(device_name)?;

        self.sample_rate = resolved.config.sample_rate;
        self.channels = resolved.config.channels;

        // Clear buffer
        {
            let mut buf = self
                .buffer
                .lock()
                .map_err(|e| AppError::Internal(e.to_string()))?;
            buf.clear();
        }

        let buffer = Arc::clone(&self.buffer);
        let stream = build_stream(&resolved, move |data: &[f32]| {
            if let Ok(mut buf) = buffer.lock() {
                buf.extend_from_slice(data);
            }
        })?;

        stream.play()?;
        self.stream = Some(stream);

        log::info!(
            "Audio capture started: {}Hz, {} channels, {:?}",
            self.sample_rate,
            self.channels,
            resolved.sample_format
        );

        Ok(())
    }

    /// Start capture in streaming mode. Returns an unbounded receiver of audio chunks.
    /// Each chunk is a `Vec<f32>` of raw samples (possibly multi-channel, native sample rate).
    pub fn start_streaming(
        &mut self,
        device_name: Option<&str>,
    ) -> Result<tokio::sync::mpsc::UnboundedReceiver<Vec<f32>>, AppError> {
        let resolved = resolve_input_device(device_name)?;

        self.sample_rate = resolved.config.sample_rate;
        self.channels = resolved.config.channels;

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Vec<f32>>();

        // In streaming mode, audio is consumed via the mpsc channel.
        // No backup buffer accumulation — avoids Mutex contention + memory waste
        // (~4KB memcpy every 10ms, up to 115MB for a 5-minute recording).
        let stream = build_stream(&resolved, move |data: &[f32]| {
            let _ = tx.send(data.to_vec());
        })?;

        stream.play()?;
        self.stream = Some(stream);

        log::info!(
            "Audio capture started (streaming): {}Hz, {} channels, {:?}",
            self.sample_rate,
            self.channels,
            resolved.sample_format
        );

        Ok(rx)
    }

    pub fn stop(&mut self) -> Result<CapturedAudio, AppError> {
        // Drop the stream to stop recording
        self.stream.take();

        let samples = {
            let mut buf = self.buffer.lock().map_err(|e| AppError::Internal(e.to_string()))?;
            std::mem::take(&mut *buf)
        };

        log::info!(
            "Audio capture stopped: {} samples ({:.1}s)",
            samples.len(),
            samples.len() as f64 / (self.sample_rate as f64 * self.channels as f64)
        );

        Ok(CapturedAudio {
            samples,
            sample_rate: self.sample_rate,
            channels: self.channels,
        })
    }

    pub fn is_recording(&self) -> bool {
        self.stream.is_some()
    }
}

pub struct CapturedAudio {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
}

pub fn list_input_devices() -> Result<Vec<InputDeviceInfo>, AppError> {
    let host = cpal::default_host();
    let default_name = host
        .default_input_device()
        .and_then(|d| d.description().ok())
        .map(|desc| desc.name().to_string());

    let mut devices = Vec::new();
    for device in host.input_devices()? {
        if let Ok(desc) = device.description() {
            let name = desc.name().to_string();
            let is_default = default_name.as_deref() == Some(name.as_str());
            devices.push(InputDeviceInfo { name, is_default });
        }
    }
    Ok(devices)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct InputDeviceInfo {
    pub name: String,
    pub is_default: bool,
}
