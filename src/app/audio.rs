use super::*;
use cpal::traits::StreamTrait;
use rustfft::num_complex::Complex;
use rustfft::FftPlanner;
use std::time::{Duration, Instant};

impl Js8App {
    /// Starts the audio stream for the selected input device.
    pub fn start_audio_stream(&mut self) {
        println!("Starting audio stream...");

        let device = self.devices[self.selected_device_index].clone();
        println!("Device: {:?}", device.default_input_config());
        let config = match device.default_input_config() {
            Ok(config) => config,
            Err(err) => {
                eprintln!("Failed to get default input config: {}", err);
                return;
            }
        };

        // Ensure the sample rate matches 48 kHz
        if config.sample_rate().0 != self.sample_rate as u32 {
            eprintln!(
                "Warning: The device sample rate is not 48 kHz, but {} Hz",
                config.sample_rate().0
            );
        }

        // Ensure the input device has 2 channels
        if config.channels() != 2 {
            eprintln!("Warning: The input device does not have 2 channels");
        }

        let audio_data = self.audio_data.clone();
        let row_colors = self.row_colors.clone();
        let max_value = self.max_value.clone();
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(self.fft_size);
        let scratch = vec![Complex { re: 0.0, im: 0.0 }; fft.get_inplace_scratch_len()];

        let mut last_update = Instant::now();

        let input_callback = {
            let audio_data = audio_data.clone();
            let row_colors = row_colors.clone();
            let max_value = max_value.clone();
            let fft = fft.clone();
            let mut scratch = scratch.clone();
            let fft_size = self.fft_size;
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if last_update.elapsed() >= Duration::from_secs_f32(0.16) {
                    let mut audio_data = audio_data.lock().unwrap();
                    let mut row_colors = row_colors.lock().unwrap();
                    let mut max_value = max_value.lock().unwrap();
                    *max_value = Self::process_audio_data(
                        fft_size,
                        *max_value,
                        data,
                        &mut audio_data,
                        &mut row_colors,
                        &*fft,
                        &mut scratch,
                    );
                    last_update = Instant::now();
                }
            }
        };

        let error_callback = move |err| {
            eprintln!("Stream error: {}", err);
        };

        let stream =
            match device.build_input_stream(&config.into(), input_callback, error_callback, None) {
                Ok(stream) => stream,
                Err(err) => {
                    eprintln!("Failed to build input stream: {}", err);
                    return;
                }
            };

        if let Err(err) = stream.play() {
            eprintln!("Failed to play stream: {}", err);
            return;
        }

        self.stream = Some(stream);
    }

    /// Processes the incoming audio data, performs FFT, and updates the visualization.
    ///
    /// # Arguments
    ///
    /// * `global_max_value` - The global maximum value for normalization.
    /// * `data` - The incoming audio data.
    /// * `audio_data` - The shared audio data buffer.
    /// * `row_colors` - The shared color data for the rows in the visualization.
    /// * `fft` - The FFT processor.
    /// * `scratch` - The scratch buffer for FFT processing.
    ///
    /// # Returns
    ///
    /// The updated global maximum value.
    fn process_audio_data(
        fft_size: usize,
        global_max_value: f32,
        data: &[f32],
        audio_data: &mut VecDeque<f32>,
        row_colors: &mut Vec<Vec<Color32>>,
        fft: &dyn rustfft::Fft<f32>,
        scratch: &mut [Complex<f32>],
    ) -> f32 {
        // Convert stereo to mono and store in the audio buffer
        for samples in data.chunks(2) {
            if audio_data.len() == fft_size {
                audio_data.pop_front();
            }
            let mono_sample = (samples[0] + samples[1]) / 2.0;
            audio_data.push_back(mono_sample);
        }

        // Perform FFT on the audio data
        if audio_data.len() == fft_size {
            let mut buffer: Vec<Complex<f32>> = audio_data
                .iter()
                .map(|&x| Complex { re: x, im: 0.0 })
                .collect();
            fft.process_with_scratch(&mut buffer, scratch);

            // Use raw FFT values up to num_buckets
            let raw_values: Vec<f32> = buffer.iter().map(|c| c.norm()).collect();

            // Update the maximum value seen so far
            let max_value = raw_values.iter().cloned().fold(f32::MIN, f32::max);

            // Normalize the values and convert to colors
            let normalized_values: Vec<f32> = raw_values.iter().map(|&x| x / max_value).collect();

            let colors: Vec<Color32> = normalized_values
                .iter()
                .map(|&x| {
                    let intensity = (x * 255.0) as u8;
                    Color32::from_rgb(intensity, intensity, intensity)
                })
                .collect();

            // Update the row colors
            row_colors.push(colors);
            if row_colors.len() > 100 {
                row_colors.remove(0);
            }

            return max_value;
        }

        global_max_value
    }
}
