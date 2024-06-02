use super::*;

pub fn update_ui(app: &mut Js8App, ctx: &egui::Context) {
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            if ui.button("Start Audio Stream").clicked() {
                app.start_audio_stream();
            }
            ui.add_space(16.0);
            egui::widgets::global_dark_light_mode_buttons(ui);
        });
    });

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("Audio Bar Chart Visualization");

        egui::ComboBox::from_label("Select Input Device")
            .selected_text(
                app.devices[app.selected_device_index]
                    .name()
                    .unwrap()
                    .to_string(),
            )
            .show_ui(ui, |ui| {
                for (index, device) in app.devices.iter().enumerate() {
                    ui.selectable_value(
                        &mut app.selected_device_index,
                        index,
                        device.name().unwrap(),
                    );
                }
            });

        // app.draw_bar_chart(ui);
        app.draw_waterfall(ui);
    });

    ctx.request_repaint();
}
