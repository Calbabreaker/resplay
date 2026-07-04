use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{Sample, traits::StreamTrait};
use ringbuf::traits::Consumer;

pub fn setup_audio_stream(emu: &mut resplay_core::Emulator) -> Result<cpal::Stream, cpal::Error> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or(cpal::Error::new(cpal::ErrorKind::DeviceNotAvailable))?;
    let config = device.default_output_config()?;
    let state = StreamState {
        sample_cons: emu
            .setup_audio_buffer(config.sample_rate(), std::time::Duration::from_millis(100)),
        config: config.into(),
        device,
    };
    log::info!(
        "Setup audio stream with sample format {} and rate {}hz",
        config.sample_format(),
        config.sample_rate()
    );

    match config.sample_format() {
        cpal::SampleFormat::I8 => make_stream::<i8>(state),
        cpal::SampleFormat::I16 => make_stream::<i16>(state),
        cpal::SampleFormat::I24 => make_stream::<cpal::I24>(state),
        cpal::SampleFormat::I32 => make_stream::<i32>(state),
        cpal::SampleFormat::I64 => make_stream::<i64>(state),
        cpal::SampleFormat::U8 => make_stream::<u8>(state),
        cpal::SampleFormat::U16 => make_stream::<u16>(state),
        cpal::SampleFormat::U24 => make_stream::<cpal::U24>(state),
        cpal::SampleFormat::U32 => make_stream::<u32>(state),
        cpal::SampleFormat::U64 => make_stream::<u64>(state),
        cpal::SampleFormat::F32 => make_stream::<f32>(state),
        cpal::SampleFormat::F64 => make_stream::<f64>(state),
        _ => Err(cpal::Error::new(cpal::ErrorKind::UnsupportedConfig)),
    }
}

struct StreamState {
    sample_cons: ringbuf::HeapCons<f32>,
    config: cpal::StreamConfig,
    device: cpal::Device,
}

fn make_stream<T: cpal::SizedSample + cpal::FromSample<f32>>(
    mut state: StreamState,
) -> Result<cpal::Stream, cpal::Error> {
    let stream = state.device.build_output_stream(
        state.config,
        move |out: &mut [T], _| {
            for frame in out.chunks_mut(state.config.channels as usize) {
                let value = state.sample_cons.try_pop().unwrap_or(f32::EQUILIBRIUM);
                for sample in frame.iter_mut() {
                    *sample = T::from_sample(value);
                }
            }
        },
        |err| log::error!("Audio stream error: {err}"),
        None,
    )?;
    stream.play()?;
    Ok(stream)
}
