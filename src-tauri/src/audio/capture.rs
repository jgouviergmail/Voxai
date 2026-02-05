use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};

use crate::error::AppError;

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

    pub fn start(&mut self, device_name: Option<&str>) -> Result<(), AppError> {
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

        self.sample_rate = config.sample_rate;
        self.channels = config.channels;

        // Clear buffer
        {
            let mut buf = self.buffer.lock().map_err(|e| AppError::Internal(e.to_string()))?;
            buf.clear();
        }

        let buffer = Arc::clone(&self.buffer);
        let err_fn = |err: cpal::StreamError| {
            log::error!("Audio stream error: {}", err);
        };

        let stream = match sample_format {
            SampleFormat::F32 => {
                device.build_input_stream(
                    &config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        if let Ok(mut buf) = buffer.lock() {
                            buf.extend_from_slice(data);
                        }
                    },
                    err_fn,
                    None,
                )?
            }
            SampleFormat::I16 => {
                let buffer = Arc::clone(&self.buffer);
                device.build_input_stream(
                    &config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        if let Ok(mut buf) = buffer.lock() {
                            buf.extend(data.iter().map(|&s| s as f32 / i16::MAX as f32));
                        }
                    },
                    err_fn,
                    None,
                )?
            }
            SampleFormat::I32 => {
                let buffer = Arc::clone(&self.buffer);
                device.build_input_stream(
                    &config,
                    move |data: &[i32], _: &cpal::InputCallbackInfo| {
                        if let Ok(mut buf) = buffer.lock() {
                            buf.extend(data.iter().map(|&s| s as f32 / i32::MAX as f32));
                        }
                    },
                    err_fn,
                    None,
                )?
            }
            _ => {
                return Err(AppError::Audio(format!(
                    "Unsupported sample format: {:?}",
                    sample_format
                )));
            }
        };

        stream.play()?;
        self.stream = Some(stream);

        log::info!(
            "Audio capture started: {}Hz, {} channels, {:?}",
            self.sample_rate,
            self.channels,
            sample_format
        );

        Ok(())
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
