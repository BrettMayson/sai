use std::path::PathBuf;

use arma_rs::Context;
use rubato::Resampler;
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters,
    convert_stereo_to_mono_audio,
};

pub fn spoke(ctx: Context, callsign: String, path: PathBuf) {
    std::thread::spawn(move || {
        let language = "en";

        let file = hound::WavReader::open(path).unwrap();
        let channels = file.spec().channels;
        let sample_rate = file.spec().sample_rate as f64;
        let samples: Vec<f32> = file.into_samples::<f32>().map(|x| x.unwrap()).collect();

        // Convert to mono if needed
        let mono_samples = if channels == 2 {
            convert_stereo_to_mono_audio(&samples).expect("failed to convert stereo to mono")
        } else {
            samples
        };

        // Resample to 16kHz using rubato
        let target_sample_rate = 16000.0;

        // Create a resampler
        let mut resampler = rubato::FftFixedIn::<f32>::new(
            sample_rate as usize,
            target_sample_rate as usize,
            mono_samples.len(),
            2,
            1,
        )
        .expect("failed to create resampler");

        // Process the samples (rubato expects Vec<Vec<f32>> for channels)
        let input_frames = vec![mono_samples];
        let output_frames = resampler
            .process(&input_frames, None)
            .expect("failed to resample audio");
        let output = output_frames[0].clone(); // Extract the resampled mono audio

        // load a context and model
        let whisper_context = WhisperContext::new_with_params(
            "H:\\Programming\\GitHub\\brettmayson\\whisper-rs\\ggml-base.en.bin",
            WhisperContextParameters::default(),
        )
        .expect("failed to load model");

        let mut state = whisper_context
            .create_state()
            .expect("failed to create state");

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        params.set_language(Some(language));

        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        state
            .full(params, &output[..])
            .expect("failed to run model");

        let mut output = String::new();

        // fetch the results
        let num_segments = state
            .full_n_segments()
            .expect("failed to get number of segments");
        for i in 0..num_segments {
            let segment = state
                .full_get_segment_text(i)
                .expect("failed to get segment");
            let start_timestamp = state
                .full_get_segment_t0(i)
                .expect("failed to get segment start timestamp");
            let end_timestamp = state
                .full_get_segment_t1(i)
                .expect("failed to get segment end timestamp");
            println!("[{} - {}]: {}", start_timestamp, end_timestamp, segment);
            output.push_str(segment.as_str());
        }

        println!("Output: {}", output);

        if output.is_empty() {
            return;
        }

        if let Err(e) = ctx.callback_data("sai", "spoke", (callsign, output)) {
            eprintln!("Error sending callback: {}", e);
        }
    });
}
