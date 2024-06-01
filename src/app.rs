use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use cpal::{Device, Stream};
use egui::{Color32, Pos2};
use rustfft::num_complex::Complex;
use rustfft::FftPlanner;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Js8App {
    label: String,
    #[serde(skip)]
    value: f32,
    #[serde(skip)]
    audio_data: Arc<Mutex<Vec<f32>>>,
    #[serde(skip)]
    stream: Option<Stream>,
    #[serde(skip)]
    devices: Vec<Device>,
    #[serde(skip)]
    selected_device_index: usize,
    #[serde(skip)]
    fft_buffer: Arc<Mutex<VecDeque<Vec<f32>>>>,
    #[serde(skip)]
    fft_planner: FftPlanner<f32>,
    #[serde(skip)]
    row_colors: Arc<Mutex<Vec<Vec<Color32>>>>,
}

impl Default for Js8App {
    fn default() -> Self {
        let host = cpal::default_host();
        let devices: Vec<Device> = host.input_devices().unwrap().collect();
        Self {
            label: "Hello World!".to_owned(),
            value: 2.7,
            audio_data: Arc::new(Mutex::new(vec![])),
            stream: None,
            devices,
            selected_device_index: 0,
            fft_buffer: Arc::new(Mutex::new(VecDeque::new())),
            fft_planner: FftPlanner::new(),
            row_colors: Arc::new(Mutex::new(vec![])),
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
        let device = self.devices[self.selected_device_index].clone();
        let config = device.default_input_config().unwrap();

        let fft = self.fft_planner.plan_fft_forward(960); // Plan a forward FFT of size 960
        let fft_buffer = self.fft_buffer.clone();
        let audio_data = self.audio_data.clone();

        let stream = device
            .build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if data.len() == 960 {
                        let mut buffer: Vec<Complex<f32>> =
                            data.iter().map(|&x| Complex { re: x, im: 0.0 }).collect();
                        let mut scratch =
                            vec![Complex { re: 0.0, im: 0.0 }; fft.get_inplace_scratch_len()];

                        fft.process_with_scratch(&mut buffer, &mut scratch);

                        // Print the first few FFT results to the console
                        for (i, complex) in buffer.iter().take(10).enumerate() {
                            println!("FFT result[{}]: {:?}", i, complex);
                        }

                        // Store the FFT result for later use
                        {
                            let mut fft_buffer = fft_buffer.lock().unwrap();
                            fft_buffer.push_back(buffer.iter().map(|c| c.re).collect());
                        }

                        // Store the audio data for later use
                        {
                            let mut audio_data = audio_data.lock().unwrap();
                            audio_data.extend_from_slice(data);
                        }
                    } else {
                        eprintln!("Received unexpected number of samples: {}", data.len());
                    }
                },
                move |err| {
                    eprintln!("Stream error: {}", err);
                },
                None,
            )
            .unwrap();

        stream.play().unwrap();
        self.stream = Some(stream);
    }

    fn draw_waterfall(&self, ui: &mut egui::Ui) {
        let rect = ui.available_rect_before_wrap();
        let (rect_width, rect_height) = (rect.width() as usize, rect.height() as usize);

        let fft_buffer = self.fft_buffer.lock().unwrap();
        let fft_data: Vec<f32> = fft_buffer.iter().flatten().cloned().collect();
        // Limit the history of samples to the width of the rectangle
        let fft_data = if fft_data.len() > rect_width {
            &fft_data[fft_data.len() - rect_width..]
        } else {
            &fft_data[..]
        };

        // Reduce the resolution of the spectrum by drawing every nth FFT result
        let resolution = 4; // Change this value to adjust the resolution
        let max_value = fft_data.iter().cloned().fold(0.0 / 0.0, f32::max);

        // Lock the row_colors buffer
        let mut row_colors = self.row_colors.lock().unwrap();

        // Initialize the buffer if it's empty
        if row_colors.is_empty() {
            *row_colors = vec![vec![Color32::BLACK; rect_width]; rect_height];
        }

        // Shift the rows down faster by shifting more rows at a time
        let shift_amount = 2; // Increase this value to scroll faster
        for i in (shift_amount..rect_height).rev() {
            row_colors[i] = row_colors[i - shift_amount].clone();
        }

        // Fill the top rows with the current FFT data
        for j in (0..rect_width).step_by(resolution) {
            let intensity = if let Some(&value) = fft_data.get(j) {
                value / max_value // Normalize the intensity
            } else {
                0.0
            };
            let color = Self::intensity_to_color(intensity);
            for k in 0..shift_amount {
                row_colors[k][j] = color;
            }
        }

        // Draw the waterfall with taller rows
        let row_height = 2.0; // Increase this value to make rows taller
        for i in 0..rect_height {
            for j in 0..rect_width {
                ui.painter().rect_filled(
                    egui::Rect::from_min_size(
                        egui::Pos2::new(rect.min.x + j as f32, rect.min.y + i as f32 * row_height),
                        egui::Vec2::new(1.0, row_height),
                    ),
                    0.0,
                    row_colors[i][j],
                );
            }
        }

        // Draw red lines on the left and right of the spectrum
        let line_color = Color32::RED;
        let line_thickness = 2.0;

        ui.painter().line_segment(
            [
                Pos2::new(rect.min.x, rect.min.y),
                Pos2::new(rect.min.x, rect.max.y),
            ],
            (line_thickness, line_color),
        );

        ui.painter().line_segment(
            [
                Pos2::new(rect.max.x, rect.min.y),
                Pos2::new(rect.max.x, rect.max.y),
            ],
            (line_thickness, line_color),
        );
    }

    fn intensity_to_color(intensity: f32) -> Color32 {
        let intensity = intensity.clamp(0.0, 1.0);
        let r = (intensity * 255.0) as u8;
        let b = ((1.0 - intensity) * 255.0) as u8;
        Color32::from_rgb(r, 0, b)
    }
}

impl eframe::App for Js8App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Request a repaint for continuous updates
        ctx.request_repaint();

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
            ui.heading("Audio Waterfall Visualization");

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

            // Draw the waterfall visualization
            self.draw_waterfall(ui);
        });
    }
}
