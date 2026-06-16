use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

use eframe::egui;
use quarian_voice_filter::{process_wav_bytes, QuarianVoiceFilterParams};

type ProcessResult = Result<Vec<u8>, String>;

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

fn presets() -> Vec<(&'static str, QuarianVoiceFilterParams)> {
    vec![
        ("Default", QuarianVoiceFilterParams::default()),
        (
            "Subtle",
            QuarianVoiceFilterParams {
                pitch_semitones: 0.0,
                dry_gain: 0.5,
                wet_gain: 0.08,
                hpf: 100.0,
                lpf: 12000.0,
                notch: 1500.0,
                drive: 0.02,
            },
        ),
        (
            "Heavy",
            QuarianVoiceFilterParams {
                pitch_semitones: 2.0,
                dry_gain: 0.1,
                wet_gain: 0.25,
                hpf: 300.0,
                lpf: 5000.0,
                notch: 800.0,
                drive: 0.12,
            },
        ),
        (
            "Radio comm",
            QuarianVoiceFilterParams {
                pitch_semitones: 0.5,
                dry_gain: 0.0,
                wet_gain: 0.3,
                hpf: 400.0,
                lpf: 3500.0,
                notch: 1200.0,
                drive: 0.2,
            },
        ),
    ]
}

struct App {
    params: QuarianVoiceFilterParams,
    input_path: Option<PathBuf>,
    input_info: String,
    output_bytes: Option<Vec<u8>>,
    status: String,
    processing: bool,
    pending_result: Option<Arc<Mutex<Option<ProcessResult>>>>,
    selected_preset: usize,
    dark_theme_set: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            params: QuarianVoiceFilterParams::default(),
            input_path: None,
            input_info: String::new(),
            output_bytes: None,
            status: String::new(),
            processing: false,
            pending_result: None,
            selected_preset: 0,
            dark_theme_set: false,
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if !self.dark_theme_set {
            ui.ctx().set_visuals(egui::Visuals::dark());
            self.dark_theme_set = true;
        }

        self.check_pending_result();

        if self.processing {
            ui.ctx().request_repaint();
        }

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
        ui.horizontal(|ui| {
            ui.label("Preset:");
            egui::ComboBox::from_id_salt("preset")
                .selected_text(presets()[self.selected_preset].0)
                .show_ui(ui, |ui| {
                    for (i, (name, _)) in presets().iter().enumerate() {
                        if ui
                            .selectable_label(i == self.selected_preset, *name)
                            .clicked()
                        {
                            self.selected_preset = i;
                            self.params = presets()[i].1;
                        }
                    }
                });
        });

        ui.separator();
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
        if ui
            .add_enabled(
                self.selected_preset != 0,
                egui::Button::new("Reset defaults"),
            )
            .clicked()
        {
            self.selected_preset = 0;
            self.params = QuarianVoiceFilterParams::default();
        }
    }

    fn action_section(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let has_input = self.input_path.is_some();
            let can_process = has_input && !self.processing;

            if ui
                .add_enabled(can_process, egui::Button::new("Process"))
                .clicked()
            {
                self.start_process();
            }

            if self.processing {
                ui.add(egui::Spinner::new());
            }

            if ui
                .add_enabled(
                    self.output_bytes.is_some() && !self.processing,
                    egui::Button::new("Save WAV"),
                )
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

    fn start_process(&mut self) {
        let path = match &self.input_path {
            Some(p) => p.clone(),
            None => return,
        };

        let params = self.params;

        self.output_bytes = None;
        self.processing = true;
        self.status = "Processing...".into();

        let result_holder: Arc<Mutex<Option<ProcessResult>>> = Arc::new(Mutex::new(None));
        let holder = Arc::clone(&result_holder);

        thread::spawn(move || {
            let t0 = std::time::Instant::now();
            let outcome = match std::fs::read(&path) {
                Ok(input_bytes) => {
                    process_wav_bytes(&input_bytes, &params).map_err(|e| format!("{e}"))
                }
                Err(e) => Err(format!("Error reading file: {e}")),
            };
            let _elapsed_ms = t0.elapsed().as_secs_f64() * 1000.0;

            *holder.lock().unwrap() = Some(outcome);
        });

        self.pending_result = Some(result_holder);
    }

    fn check_pending_result(&mut self) {
        let Some(holder) = self.pending_result.take() else {
            return;
        };

        let outcome = {
            let mut result = holder.lock().unwrap();
            result.take()
        };

        match outcome {
            Some(Ok(output)) => {
                self.output_bytes = Some(output);
                self.status = "Done.".into();
                self.processing = false;
            }
            Some(Err(e)) => {
                self.status = format!("Error: {e}");
                self.processing = false;
            }
            None => {
                self.pending_result = Some(holder);
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
