pub mod audio;
pub mod ui;
pub mod visualization;

use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{Device, Stream};

use egui::Color32;

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

const FFT_SIZE: usize = 2400;
const SAMPLE_RATE: f32 = 48000.0;
const MAX_FREQUENCY: f32 = 3000.0;

/// The main application structure for JS8App.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Js8App {
    /// Shared audio data buffer.
    #[serde(skip)]
    audio_data: Arc<Mutex<VecDeque<f32>>>,
    /// Optional audio stream.
    #[serde(skip)]
    stream: Option<Stream>,
    /// List of available audio input devices.
    #[serde(skip)]
    devices: Vec<Device>,
    /// Index of the selected audio input device.
    #[serde(skip)]
    selected_device_index: usize,
    /// Shared color data for the rows in the visualization.
    #[serde(skip)]
    row_colors: Arc<Mutex<Vec<Vec<Color32>>>>,
    /// Minimum value for normalization.
    #[serde(skip)]
    min_value: f32,
    /// Shared maximum value for normalization.
    #[serde(skip)]
    max_value: Arc<Mutex<f32>>,
}

impl Default for Js8App {
    /// Creates a default instance of `Js8App`.
    fn default() -> Self {
        let host = cpal::default_host();
        let devices: Vec<Device> = host.input_devices().unwrap().collect();
        Self {
            audio_data: Arc::new(Mutex::new(VecDeque::with_capacity(FFT_SIZE))),
            stream: None,
            devices,
            selected_device_index: 0,
            row_colors: Arc::new(Mutex::new(vec![])),
            min_value: 0.0,
            max_value: Arc::new(Mutex::new(0.0)),
        }
    }
}

impl Js8App {
    /// Creates a new instance of `Js8App` with the given creation context.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }
        Default::default()
    }
}

impl eframe::App for Js8App {
    /// Saves the current state of the application.
    ///
    /// # Arguments
    ///
    /// * `storage` - The storage to save the state to.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Updates the application state and UI.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The egui context.
    /// * `_frame` - The eframe frame.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ui::update_ui(self, ctx);
    }
}
