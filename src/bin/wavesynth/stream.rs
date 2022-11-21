use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{Device, StreamConfig};
use std::sync::Arc;

use wavetable::system::System;

pub fn make_stream<F>(system: &Arc<System>, perform: F) -> anyhow::Result<cpal::Stream>
where
    F: FnMut(&mut [f32], &cpal::OutputCallbackInfo) + std::marker::Send + 'static,
{
    let host = cpal::default_host();
    let device = host.default_output_device().ok_or_else(|| {
        anyhow::Error::msg(format!("No default device for {} host", host.id().name()))
    })?;

    let config = get_config(system, &device).ok_or_else(|| {
        anyhow::Error::msg(format!(
            "No supported stream configuration for {}Hz sample rate and {} bufsize.",
            system.samplerate(),
            system.bufsize()
        ))
    })?;

    println!("Creating stream for device: {}", device.name().unwrap());

    device
        .build_output_stream(&config, perform, |err| {
            eprintln!("Error building output stream {}", err)
        })
        .map_err(|_| anyhow::Error::msg("Unable to build stream"))
}

pub fn get_config(system: &Arc<System>, device: &Device) -> Option<StreamConfig> {
    let configs = device
        .supported_output_configs()
        .expect("Attempted to get configs from invalid device");
    for range in configs {
        let fs = system.samplerate() as u32;
        if fs < range.min_sample_rate().0 || fs > range.max_sample_rate().0 {
            continue;
        }

        let bufsize = system.bufsize() as u32;
        if let cpal::SupportedBufferSize::Range { min, max } = range.buffer_size() {
            if bufsize < *min || bufsize > *max {
                continue;
            }
        } else {
            continue;
        }

        if range.sample_format() != cpal::SampleFormat::F32 {
            continue;
        }

        let mut config = range.with_sample_rate(cpal::SampleRate(fs)).config();
        config.buffer_size = cpal::BufferSize::Fixed(bufsize);
        config.channels = 1;
        return Some(config);
    }

    None
}
