use super::*;

impl Js8App {
    /// Draws a bar chart visualization of the audio data.
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI context.
    pub fn draw_bar_chart(&self, ui: &mut egui::Ui) {
        let row_colors = self.row_colors.lock().unwrap();
        if row_colors.is_empty() {
            return;
        }

        // Calculate the fraction of the spectrum to display
        let spectrum_fraction = MAX_FREQUENCY / SAMPLE_RATE;
        let num_buckets = spectrum_fraction * FFT_SIZE as f32;
        let bar_width = ui.available_width() / num_buckets;
        let max_height = ui.available_height();

        let painter = ui.painter();

        // Draw each bar in the bar chart
        for (i, &color) in row_colors[0]
            .iter()
            .take(num_buckets.ceil() as usize)
            .enumerate()
        {
            let value = color.r() as f32 / 255.0;
            let height = max_height * value;
            let rect = egui::Rect::from_min_size(
                egui::pos2(i as f32 * bar_width, max_height - height),
                egui::vec2(bar_width, height),
            );
            painter.rect_filled(rect, 0.0, color);
        }
    }
}
