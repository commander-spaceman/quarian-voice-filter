use eframe::egui;
use quarian_voice_filter::QuarianVoiceFilterParams;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([520.0, 560.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Quarian Voice Filter",
        options,
        Box::new(|_cc| Ok(Box::new(App::default()))),
    )
}

struct App {
    params: QuarianVoiceFilterParams,
}

impl Default for App {
    fn default() -> Self {
        Self {
            params: QuarianVoiceFilterParams::default(),
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("Quarian Voice Filter");
            ui.separator();

            ui.add_space(8.0);
            ui.collapsing("Pitch", |ui| {
                ui.add(
                    egui::Slider::new(&mut self.params.pitch_semitones, -12.0..=12.0)
                        .text("Semitones"),
                );
                if ui.button("0 (no shift)").clicked() {
                    self.params.pitch_semitones = 0.0;
                }
            });

            ui.add_space(8.0);
            ui.collapsing("Filters", |ui| {
                ui.add(
                    egui::Slider::new(&mut self.params.hpf, 20.0..=2000.0).text("High-pass (Hz)"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.lpf, 1000.0..=20000.0).text("Low-pass (Hz)"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.notch, 100.0..=5000.0).text("Notch (Hz)"),
                );
            });

            ui.add_space(8.0);
            ui.collapsing("Mix & Saturation", |ui| {
                ui.add(egui::Slider::new(&mut self.params.dry_gain, 0.0..=1.0).text("Dry gain"));
                ui.add(egui::Slider::new(&mut self.params.wet_gain, 0.0..=1.0).text("Wet gain"));
                ui.add(egui::Slider::new(&mut self.params.drive, 0.0..=1.0).text("Drive"));
            });

            ui.add_space(16.0);
            if ui.button("Reset defaults").clicked() {
                self.params = QuarianVoiceFilterParams::default();
            }
        });
    }
}
