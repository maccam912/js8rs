use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Stream};

use egui::Color32;

use rustfft::{num_complex::Complex, FftPlanner};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

const FFT_SIZE: usize = 4800;
const SAMPLE_RATE: f32 = 48000.0;
const MAX_FREQUENCY: f32 = 3000.0;
const BUCKET_WIDTH: f32 = SAMPLE_RATE / FFT_SIZE as f32;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Js8App {
    label: String,
    #[serde(skip)]
    value: f32,
    #[serde(skip)]
    audio_data: Arc<Mutex<VecDeque<f32>>>,
    #[serde(skip)]
    stream: Option<Stream>,
    #[serde(skip)]
    devices: Vec<Device>,
    #[serde(skip)]
    selected_device_index: usize,
    #[serde(skip)]
    row_colors: Arc<Mutex<Vec<Vec<Color32>>>>,
    #[serde(skip)]
    min_value: f32,
    #[serde(skip)]
    max_value: Arc<Mutex<f32>>,
    #[serde(skip)]
    num_buckets: usize,
}

impl Default for Js8App {
    fn default() -> Self {
        let host = cpal::default_host();
        let devices: Vec<Device> = host.input_devices().unwrap().collect();
        let num_buckets = (MAX_FREQUENCY / BUCKET_WIDTH).ceil() as usize;
        Self {
            label: "Hello World!".to_owned(),
            value: 2.7,
            audio_data: Arc::new(Mutex::new(VecDeque::with_capacity(FFT_SIZE))),
            stream: None,
            devices,
            selected_device_index: 0,
            row_colors: Arc::new(Mutex::new(vec![])),
            min_value: 0.0,
            max_value: Arc::new(Mutex::new(0.0)),
            num_buckets,
        }
    }
}

impl Js8App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }
        Default::default()
    }

    fn start_audio_stream(&mut self) {
        println!("Starting audio stream...");

        let device = self.devices[self.selected_device_index].clone();
        let config = match device.default_input_config() {
            Ok(config) => config,
            Err(err) => {
                eprintln!("Failed to get default input config: {}", err);
                return;
            }
        };

        // Ensure the sample rate matches 48 kHz
        if config.sample_rate().0 != SAMPLE_RATE as u32 {
            eprintln!(
                "Warning: The device sample rate is not 48 kHz, but {} Hz",
                config.sample_rate().0
            );
        }

        let audio_data = self.audio_data.clone();
        let row_colors = self.row_colors.clone();
        let max_value = self.max_value.clone();
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);
        let scratch = vec![Complex { re: 0.0, im: 0.0 }; fft.get_inplace_scratch_len()];

        let num_buckets = self.num_buckets; // Capture num_buckets

        let stream = match device.build_input_stream(
            &config.into(),
            {
                let audio_data = audio_data.clone();
                let row_colors = row_colors.clone();
                let max_value = max_value.clone();
                let fft = fft.clone();
                let mut scratch = scratch.clone();
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let mut audio_data = audio_data.lock().unwrap();
                    let mut row_colors = row_colors.lock().unwrap();
                    let mut max_value = max_value.lock().unwrap();
                    *max_value = Self::process_audio_data(
                        *max_value,
                        data,
                        &mut audio_data,
                        &mut row_colors,
                        &*fft,
                        &mut scratch,
                        num_buckets,
                    );
                }
            },
            move |err| {
                eprintln!("Stream error: {}", err);
            },
            None,
        ) {
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

    fn process_audio_data(
        global_max_value: f32,
        data: &[f32],
        audio_data: &mut VecDeque<f32>,
        row_colors: &mut Vec<Vec<Color32>>,
        fft: &dyn rustfft::Fft<f32>,
        scratch: &mut [Complex<f32>],
        num_buckets: usize,
    ) -> f32 {
        for &sample in data {
            if audio_data.len() == FFT_SIZE {
                audio_data.pop_front();
            }
            audio_data.push_back(sample);
        }

        // Perform FFT on the audio data
        if audio_data.len() == FFT_SIZE {
            let mut buffer: Vec<Complex<f32>> = audio_data
                .iter()
                .map(|&x| Complex { re: x, im: 0.0 })
                .collect();
            fft.process_with_scratch(&mut buffer, scratch);

            // Use raw FFT values
            let raw_values: Vec<f32> = buffer.iter().take(num_buckets).map(|c| c.norm()).collect();

            // Update the maximum value seen so far
            let max_value = raw_values.iter().cloned().fold(f32::MIN, f32::max);

            // Update row_colors with scaled values
            row_colors.clear();
            row_colors.push(
                raw_values
                    .iter()
                    .map(|&v| {
                        let scaled_value = v / global_max_value;
                        let intensity = (scaled_value * 255.0) as u8;
                        Color32::from_rgb(intensity, 0, 0) // Store value in the red channel
                    })
                    .collect(),
            );

            // println!("Updated row_colors: {:?}", row_colors);

            if max_value > global_max_value {
                return max_value;
            } else {
                return global_max_value;
            }
        }
        global_max_value
    }

    fn draw_bar_chart(&self, ui: &mut egui::Ui) {
        let row_colors = self.row_colors.lock().unwrap();
        if row_colors.is_empty() {
            return;
        }

        let bar_width = ui.available_width() / self.num_buckets as f32;
        let max_height = ui.available_height();

        let painter = ui.painter();

        for (i, &color) in row_colors[0].iter().enumerate() {
            let height = max_height * (color.r() as f32 / 255.0);
            let rect = egui::Rect::from_min_size(
                egui::pos2(i as f32 * bar_width, max_height - height),
                egui::vec2(bar_width, height),
            );
            painter.rect_filled(rect, 0.0, color);
        }
    }
}

impl eframe::App for Js8App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                if ui.button("Start Audio Stream").clicked() {
                    self.start_audio_stream();
                }
                ui.add_space(16.0);
                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Audio Bar Chart Visualization");

            // Dropdown for selecting the input device
            egui::ComboBox::from_label("Select Input Device")
                .selected_text(
                    self.devices[self.selected_device_index]
                        .name()
                        .unwrap()
                        .to_string(),
                )
                .show_ui(ui, |ui| {
                    for (index, device) in self.devices.iter().enumerate() {
                        ui.selectable_value(
                            &mut self.selected_device_index,
                            index,
                            device.name().unwrap(),
                        );
                    }
                });

            // Draw the bar chart
            self.draw_bar_chart(ui);
        });

        // Request a repaint to keep the UI updated
        ctx.request_repaint();
    }
}
