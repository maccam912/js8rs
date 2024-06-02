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

    /// Draws a waterfall visualization of the audio data.
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI context.
    pub fn draw_waterfall(&self, ui: &mut egui::Ui) {
        let mut row_colors = self.row_colors.lock().unwrap();
        if row_colors.is_empty() {
            return;
        }

        // Define the number of rows to display
        let max_rows_to_display = 100;
        let num_rows = row_colors.len().min(max_rows_to_display);
        let row_height = ui.available_height() / max_rows_to_display as f32;
        let row_width = ui.available_width();

        // Calculate the fraction of the spectrum to display
        let spectrum_fraction = MAX_FREQUENCY / SAMPLE_RATE;
        let num_buckets = (spectrum_fraction * FFT_SIZE as f32).ceil() as usize;

        let painter = ui.painter();

        // Draw each row in the waterfall chart
        for (row_index, row) in row_colors.iter().rev().take(num_rows).enumerate() {
            let y_offset = row_index as f32 * row_height;

            for (col_index, &color) in row.iter().take(num_buckets).enumerate() {
                let value = color.r() as f32 / 255.0;
                let color = egui::Color32::from_rgb(
                    (value * 255.0) as u8,
                    0,
                    ((1.0 - value) * 255.0) as u8,
                );

                let rect = egui::Rect::from_min_size(
                    egui::pos2(col_index as f32 * row_width / num_buckets as f32, y_offset),
                    egui::vec2(row_width / num_buckets as f32, row_height),
                );
                painter.rect_filled(rect, 0.0, color);
            }
        }

        // Remove old rows that are no longer displayed
        let row_colors_len = row_colors.len();
        if row_colors_len > max_rows_to_display {
            row_colors.drain(0..(row_colors_len - max_rows_to_display));
        }
    }
}
