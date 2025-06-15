mod speak;
mod spoke;

use std::{
    fs::File,
    io::BufWriter,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use arma_rs::{Context, ContextState, Group};
use cpal::{
    FromSample, Sample,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};

use crate::client::speak::cmd_speak;

pub fn group() -> Group {
    Group::new()
        .command("start", cmd_start)
        .command("stop", cmd_stop)
        .command("speak", cmd_speak)
}

struct CurrentRecording {
    state: Arc<Mutex<Option<CurrentRecordingInner>>>,
}

impl CurrentRecording {
    pub fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set(&self, state: CurrentRecordingInner) {
        let mut guard = self.state.lock().unwrap();
        *guard = Some(state);
    }

    pub fn get(&self) -> Option<CurrentRecordingInner> {
        let inner = {
            let mut guard = self.state.lock().unwrap();
            guard.take()
        };
        inner.and_then(|inner| {
            inner.sender.send(()).ok()?;
            Some(inner)
        })
    }
}

struct CurrentRecordingInner {
    path: PathBuf,
    sender: std::sync::mpsc::Sender<()>,
}

#[allow(clippy::needless_pass_by_value)]
/// The user has started speaking
fn cmd_start(ctx: Context) {
    let current = ctx.group().get::<CurrentRecording>().unwrap_or_else(|| {
        ctx.group().set(CurrentRecording::default());
        ctx.group().get::<CurrentRecording>().unwrap()
    });
    let (tx, rx) = std::sync::mpsc::channel();
    let path = std::env::temp_dir().join("sai_recording.wav");
    let path2 = path.clone();
    std::thread::spawn(move || {
        if let Err(e) = record_thread(&rx, path2.as_path()) {
            eprintln!("Error recording: {e}");
        }
    });
    let state = CurrentRecordingInner { path, sender: tx };
    current.set(state);
}

fn cmd_stop(ctx: Context, callsign: String) -> Result<(), String> {
    let current = ctx
        .group()
        .get::<CurrentRecording>()
        .ok_or_else(|| "No recording in progress".to_string())?
        .get()
        .ok_or_else(|| "No recording in progress".to_string())?;
    let path = current.path;
    println!("Stopping recording at {}", path.display());

    spoke::spoke(ctx, path, callsign);

    Ok(())
}

fn record_thread(rx: &std::sync::mpsc::Receiver<()>, path: &Path) -> Result<(), String> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| "No input device available".to_string())?;
    let config = device.default_input_config().map_err(|e| e.to_string())?;

    let spec = wav_spec_from_config(&config);
    let writer = hound::WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    let writer = Arc::new(Mutex::new(Some(writer)));

    // A flag to indicate that recording is in progress.
    println!("Begin recording...");

    // Run the input stream on a separate thread.
    let writer_2 = writer;

    let err_fn = move |err| {
        eprintln!("an error occurred on stream: {err}");
    };

    let stream = match config.sample_format() {
        cpal::SampleFormat::I8 => device
            .build_input_stream(
                &config.into(),
                move |data, _: &_| write_input_data::<i8, i8>(data, &writer_2),
                err_fn,
                None,
            )
            .map_err(|e| e.to_string())?,
        cpal::SampleFormat::I16 => device
            .build_input_stream(
                &config.into(),
                move |data, _: &_| write_input_data::<i16, i16>(data, &writer_2),
                err_fn,
                None,
            )
            .map_err(|e| e.to_string())?,
        cpal::SampleFormat::I32 => device
            .build_input_stream(
                &config.into(),
                move |data, _: &_| write_input_data::<i32, i32>(data, &writer_2),
                err_fn,
                None,
            )
            .map_err(|e| e.to_string())?,
        cpal::SampleFormat::F32 => device
            .build_input_stream(
                &config.into(),
                move |data, _: &_| write_input_data::<f32, f32>(data, &writer_2),
                err_fn,
                None,
            )
            .map_err(|e| e.to_string())?,
        sample_format => return Err(format!("Unsupported sample format: {sample_format:?}")),
    };

    stream.play().map_err(|e| e.to_string())?;

    println!("Recording...");

    // Wait for the recording to finish or be stopped.
    rx.recv().map_err(|e| e.to_string())?;
    println!("Recording stopped.");

    Ok(())
}

fn sample_format(format: cpal::SampleFormat) -> hound::SampleFormat {
    if format.is_float() {
        hound::SampleFormat::Float
    } else {
        hound::SampleFormat::Int
    }
}

#[allow(clippy::cast_possible_truncation)]
fn wav_spec_from_config(config: &cpal::SupportedStreamConfig) -> hound::WavSpec {
    hound::WavSpec {
        channels: config.channels() as _,
        sample_rate: config.sample_rate().0 as _,
        bits_per_sample: (config.sample_format().sample_size() * 8) as _,
        sample_format: sample_format(config.sample_format()),
    }
}

type WavWriterHandle = Arc<Mutex<Option<hound::WavWriter<BufWriter<File>>>>>;

fn write_input_data<T, U>(input: &[T], writer: &WavWriterHandle)
where
    T: Sample,
    U: Sample + hound::Sample + FromSample<T>,
{
    if let Ok(mut guard) = writer.try_lock() {
        if let Some(writer) = guard.as_mut() {
            for &sample in input {
                let sample: U = U::from_sample(sample);
                writer.write_sample(sample).ok();
            }
        }
    }
}
