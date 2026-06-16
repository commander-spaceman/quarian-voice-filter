use std::path::PathBuf;

use eframe::egui;
use quarian_voice_filter::{process_wav_bytes, QuarianVoiceFilterParams};

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([520.0, 620.0]),
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
    input_path: Option<PathBuf>,
    input_info: String,
    output_bytes: Option<Vec<u8>>,
    status: String,
}

impl Default for App {
    fn default() -> Self {
        Self {
            params: QuarianVoiceFilterParams::default(),
            input_path: None,
            input_info: String::new(),
            output_bytes: None,
            status: String::new(),
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.file_section(ui);
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            self.param_section(ui);
        });

        ui.separator();
        self.action_section(ui);
    }
}

impl App {
    fn file_section(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Input file:");
            if let Some(ref path) = self.input_path {
                ui.label(
                    path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                );
            } else {
                ui.label("(none)");
            }
        });

        if ui.button("Open WAV").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("WAV files", &["wav"])
                .pick_file()
            {
                self.load_file(&path);
            }
        }

        if !self.input_info.is_empty() {
            ui.label(&self.input_info);
        }
    }

    fn param_section(&mut self, ui: &mut egui::Ui) {
        ui.heading("Parameters");

        ui.add_space(8.0);
        ui.collapsing("Pitch", |ui| {
            ui.add(
                egui::Slider::new(&mut self.params.pitch_semitones, -12.0..=12.0).text("Semitones"),
            );
            if ui.button("0 (no shift)").clicked() {
                self.params.pitch_semitones = 0.0;
            }
        });

        ui.add_space(8.0);
        ui.collapsing("Filters", |ui| {
            ui.add(egui::Slider::new(&mut self.params.hpf, 20.0..=2000.0).text("High-pass (Hz)"));
            ui.add(egui::Slider::new(&mut self.params.lpf, 1000.0..=20000.0).text("Low-pass (Hz)"));
            ui.add(egui::Slider::new(&mut self.params.notch, 100.0..=5000.0).text("Notch (Hz)"));
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
    }

    fn action_section(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let has_input = self.input_path.is_some();

            if ui
                .add_enabled(has_input, egui::Button::new("Process"))
                .clicked()
            {
                self.process();
            }

            if ui
                .add_enabled(self.output_bytes.is_some(), egui::Button::new("Save WAV"))
                .clicked()
            {
                self.save_output();
            }
        });

        if !self.status.is_empty() {
            ui.label(&self.status);
        }
    }

    fn load_file(&mut self, path: &PathBuf) {
        self.input_path = Some(path.clone());
        self.output_bytes = None;
        self.status.clear();

        match std::fs::read(path) {
            Ok(bytes) => match quarian_voice_filter::wav::decode_wav_bytes(&bytes) {
                Ok(mono) => {
                    let duration = mono.samples.len() as f64 / mono.sample_rate as f64;
                    self.input_info = format!(
                        "{} Hz, {} ch, {:.1}s, {} samples",
                        mono.sample_rate,
                        mono.channels,
                        duration,
                        mono.samples.len()
                    );
                    self.status = "File loaded.".into();
                }
                Err(e) => {
                    self.input_info = format!("Error: {e}");
                    self.status = "Failed to decode WAV.".into();
                }
            },
            Err(e) => {
                self.input_info = format!("Error: {e}");
                self.status = "Failed to read file.".into();
            }
        }
    }

    fn process(&mut self) {
        let path = match &self.input_path {
            Some(p) => p,
            None => return,
        };

        self.output_bytes = None;
        self.status = "Processing...".into();

        let input_bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                self.status = format!("Error reading file: {e}");
                return;
            }
        };

        let t0 = std::time::Instant::now();
        match process_wav_bytes(&input_bytes, &self.params) {
            Ok(output) => {
                let elapsed = t0.elapsed().as_secs_f64() * 1000.0;
                self.output_bytes = Some(output);
                self.status = format!("Done in {elapsed:.0} ms.");
            }
            Err(e) => {
                self.status = format!("Error: {e}");
            }
        }
    }

    fn save_output(&mut self) {
        let output = match &self.output_bytes {
            Some(b) => b,
            None => return,
        };

        if let Some(path) = rfd::FileDialog::new()
            .add_filter("WAV files", &["wav"])
            .save_file()
        {
            match std::fs::write(&path, output) {
                Ok(()) => {
                    self.status = format!(
                        "Saved to {}",
                        path.file_name().unwrap_or_default().to_string_lossy()
                    );
                }
                Err(e) => {
                    self.status = format!("Error saving file: {e}");
                }
            }
        }
    }
}
