use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Stream};
use egui::{Color32, Pos2};
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
        let device = &self.devices[self.selected_device_index];
        let config = device.default_input_config().unwrap();

        let audio_data = self.audio_data.clone();

        let stream = device.build_input_stream(
            &config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if !data.is_empty() {
                    let mut audio_data = audio_data.lock().unwrap();
                    audio_data.extend_from_slice(data);
                    if audio_data.len() > 1024 * 1024 {
                        audio_data.drain(..data.len());
                    }
                }
            },
            move |err| {
                eprintln!("Stream error: {}", err);
            },
            None,
        ).unwrap();

        stream.play().unwrap();
        self.stream = Some(stream);
    }

    fn draw_waterfall(&self, ui: &mut egui::Ui) {
        let audio_data = self.audio_data.lock().unwrap();

        let width = ui.available_width();
        let height = ui.available_height();
        let rect = ui.available_rect_before_wrap();

        ui.painter().rect_filled(rect, 0.0, Color32::BLACK);

        let sample_rate = 44100; // Assuming a sample rate of 44.1 kHz
        let num_samples = std::cmp::min(audio_data.len(), sample_rate); // Limit to the last second of data

        if num_samples > 0 {
            let step = width / num_samples as f32;
            let start_index = audio_data.len().saturating_sub(sample_rate);
            for (i, &sample) in audio_data[start_index..].iter().enumerate() {
                let x = rect.min.x + i as f32 * step;
                let y = rect.min.y + height / 2.0 - sample * height / 2.0;
                ui.painter().line_segment(
                    [Pos2::new(x, rect.min.y + height / 2.0), Pos2::new(x, y)],
                    (1.0, Color32::WHITE),
                );
            }
        }
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
                .selected_text(self.devices[self.selected_device_index].name().unwrap().to_string())
                .show_ui(ui, |ui| {
                    for (index, device) in self.devices.iter().enumerate() {
                        ui.selectable_value(&mut self.selected_device_index, index, device.name().unwrap());
                    }
                });

            // Add some space between the dropdown and the waterfall
            ui.add_space(16.0);

            // Draw the waterfall visualization
            self.draw_waterfall(ui);
        });
    }
}
