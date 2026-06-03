use aethel_gui::prelude::*;

fn main() -> Result {
    let mut enabled = true;
    let mut exposure = 0.65;
    let mut name = String::from("Nova");
    let mut clicks = 0u32;

    AethelGui::new()
        .title("AethelGUI Basic Widgets")
        .size(960, 640)
        .run_ui(move |ui| {
            ui.panel_with("main", [420.0, 390.0], |ui| {
                ui.label_styled("AethelGUI 0.3 Preview", 28.0, [0.25, 0.72, 1.0, 1.0]);
                ui.label("A retained-first runtime with immediate-style ergonomics.");
                ui.separator();

                if ui.button("Apply").clicked() {
                    clicks = clicks.saturating_add(1);
                }
                ui.checkbox("Enable preview option", &mut enabled);
                ui.slider("Exposure", &mut exposure, 0.0..=1.0);
                ui.text_input("Name", &mut name);
                ui.progress_bar("Exposure progress", exposure);

                ui.label_styled(format!("Clicks: {clicks}"), 16.0, [0.72, 0.76, 0.84, 1.0]);
            });
        })
}
