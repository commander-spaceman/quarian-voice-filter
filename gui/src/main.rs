use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

use eframe::egui::{self, Color32, RichText};
use quarian_voice_filter::{process_wav_bytes, QuarianVoiceFilterParams};

type ProcessResult = Result<Vec<u8>, String>;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([480.0, 720.0]),
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

fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    fonts.font_data.insert(
        "JetBrainsMono".to_owned(),
        egui::FontData::from_static(include_bytes!("../../assets/JetBrainsMono-Regular.ttf"))
            .into(),
    );

    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "JetBrainsMono".to_owned());

    ctx.set_fonts(fonts);
}

enum StatusKind {
    Info,
    Success,
    Error,
}

struct App {
    params: QuarianVoiceFilterParams,
    input_path: Option<PathBuf>,
    input_info: String,
    output_bytes: Option<Vec<u8>>,
    status: String,
    status_kind: StatusKind,
    processing: bool,
    pending_result: Option<Arc<Mutex<Option<ProcessResult>>>>,
    selected_preset: usize,
    initialized: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            params: QuarianVoiceFilterParams::default(),
            input_path: None,
            input_info: String::new(),
            output_bytes: None,
            status: String::new(),
            status_kind: StatusKind::Info,
            processing: false,
            pending_result: None,
            selected_preset: 0,
            initialized: false,
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if !self.initialized {
            ui.ctx().set_visuals(egui::Visuals::dark());
            setup_fonts(ui.ctx());
            self.initialized = true;
        }

        self.check_pending_result();

        if self.processing {
            ui.ctx().request_repaint();
        }

        let spacing = ui.spacing_mut();
        spacing.item_spacing = egui::vec2(8.0, 8.0);
        spacing.button_padding = egui::vec2(12.0, 6.0);

        egui::Panel::bottom("action_bar")
            .min_size(48.0)
            .show_inside(ui, |ui| {
                ui.add_space(4.0);
                self.action_bar(ui);
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("main_scroll")
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading(RichText::new("Quarian Voice Filter").size(20.0));
                    });
                    ui.add_space(4.0);

                    self.file_card(ui);
                    ui.add_space(12.0);
                    self.preset_selector(ui);
                    ui.add_space(12.0);
                    self.params_grid(ui);
                });
        });
    }
}

impl App {
    fn file_card(&mut self, ui: &mut egui::Ui) {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.set_width(ui.available_width());

            ui.horizontal(|ui| {
                ui.strong("Input file");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(ref path) = self.input_path {
                        ui.label(
                            RichText::new(
                                path.file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string(),
                            )
                            .monospace(),
                        );
                    } else {
                        ui.label(RichText::new("No file selected").weak());
                    }
                });
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
                ui.label(RichText::new(&self.input_info).weak());
            }
        });
    }

    fn preset_selector(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.strong("Preset");
            ui.add_space(8.0);
            egui::ComboBox::from_id_salt("preset")
                .width(160.0)
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

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
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
            });
        });
    }

    fn params_grid(&mut self, ui: &mut egui::Ui) {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.strong("Pitch");
            ui.add_space(4.0);
            egui::Grid::new("pitch_grid")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Semitones");
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::Slider::new(&mut self.params.pitch_semitones, -12.0..=12.0)
                                .step_by(0.1),
                        );
                        ui.label(format!("{:+.1}", self.params.pitch_semitones));
                        if ui.small_button("0").clicked() {
                            self.params.pitch_semitones = 0.0;
                        }
                    });
                    ui.end_row();
                });
        });

        ui.add_space(12.0);

        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.strong("Filters");
            ui.add_space(4.0);
            egui::Grid::new("filters_grid")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("High-pass");
                    ui.horizontal(|ui| {
                        ui.add(egui::Slider::new(&mut self.params.hpf, 20.0..=2000.0).step_by(1.0));
                        ui.label(format!("{:.0} Hz", self.params.hpf));
                    });
                    ui.end_row();

                    ui.label("Low-pass");
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::Slider::new(&mut self.params.lpf, 1000.0..=20000.0).step_by(10.0),
                        );
                        ui.label(format!("{:.0} Hz", self.params.lpf));
                    });
                    ui.end_row();

                    ui.label("Notch");
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::Slider::new(&mut self.params.notch, 100.0..=5000.0).step_by(1.0),
                        );
                        ui.label(format!("{:.0} Hz", self.params.notch));
                    });
                    ui.end_row();
                });
        });

        ui.add_space(12.0);

        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.strong("Mix & Saturation");
            ui.add_space(4.0);
            egui::Grid::new("mix_grid")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Dry gain");
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::Slider::new(&mut self.params.dry_gain, 0.0..=1.0).step_by(0.01),
                        );
                        ui.label(format!("{:.2}", self.params.dry_gain));
                    });
                    ui.end_row();

                    ui.label("Wet gain");
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::Slider::new(&mut self.params.wet_gain, 0.0..=1.0).step_by(0.01),
                        );
                        ui.label(format!("{:.2}", self.params.wet_gain));
                    });
                    ui.end_row();

                    ui.label("Drive");
                    ui.horizontal(|ui| {
                        ui.add(egui::Slider::new(&mut self.params.drive, 0.0..=1.0).step_by(0.01));
                        ui.label(format!("{:.2}", self.params.drive));
                    });
                    ui.end_row();
                });
        });
    }

    fn action_bar(&mut self, ui: &mut egui::Ui) {
        let has_input = self.input_path.is_some();
        let can_process = has_input && !self.processing;

        ui.horizontal(|ui| {
            let process_btn = egui::Button::new(RichText::new("Create & Save").size(14.0))
                .min_size(egui::vec2(140.0, 32.0));

            if ui.add_enabled(can_process, process_btn).clicked() {
                self.start_process();
            }

            if self.processing {
                ui.add(egui::Spinner::new());
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if !self.status.is_empty() {
                    let color = match self.status_kind {
                        StatusKind::Success => Color32::GREEN,
                        StatusKind::Error => Color32::RED,
                        StatusKind::Info => ui.style().visuals.weak_text_color(),
                    };
                    ui.label(RichText::new(&self.status).color(color));
                }
            });
        });
    }

    fn load_file(&mut self, path: &PathBuf) {
        self.input_path = Some(path.clone());
        self.output_bytes = None;
        self.set_status("Loading file...", StatusKind::Info);

        match std::fs::read(path) {
            Ok(bytes) => match quarian_voice_filter::wav::decode_wav_bytes(&bytes) {
                Ok(mono) => {
                    let duration = mono.samples.len() as f64 / mono.sample_rate as f64;
                    let seconds = duration as u64;
                    let ms = ((duration - seconds as f64) * 1000.0) as u64;
                    let min = seconds / 60;
                    let sec = seconds % 60;
                    self.input_info = format!(
                        "{} Hz · {} ch · {}:{:02}.{:03} · {} samples",
                        mono.sample_rate,
                        mono.channels,
                        min,
                        sec,
                        ms,
                        mono.samples.len()
                    );
                    self.set_status("File loaded.", StatusKind::Success);
                }
                Err(e) => {
                    self.input_info = format!("Error: {e}");
                    self.set_status("Failed to decode WAV.", StatusKind::Error);
                }
            },
            Err(e) => {
                self.input_info = format!("Error: {e}");
                self.set_status("Failed to read file.", StatusKind::Error);
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
        self.set_status("Processing...", StatusKind::Info);

        let result_holder: Arc<Mutex<Option<ProcessResult>>> = Arc::new(Mutex::new(None));
        let holder = Arc::clone(&result_holder);

        thread::spawn(move || {
            let outcome = match std::fs::read(&path) {
                Ok(input_bytes) => {
                    process_wav_bytes(&input_bytes, &params).map_err(|e| format!("{e}"))
                }
                Err(e) => Err(format!("Error reading file: {e}")),
            };

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
                self.processing = false;
                self.set_status("Done.", StatusKind::Success);
                self.save_output();
            }
            Some(Err(e)) => {
                self.set_status(&e, StatusKind::Error);
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

        let now = chrono::Local::now();
        let default_name = format!("output_{}.wav", now.format("%Y%m%d_%H%M%S_%3f"));
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("WAV files", &["wav"])
            .set_file_name(default_name)
            .save_file()
        {
            match std::fs::write(&path, output) {
                Ok(()) => {
                    self.set_status(
                        &format!(
                            "Saved to {}",
                            path.file_name().unwrap_or_default().to_string_lossy()
                        ),
                        StatusKind::Success,
                    );
                }
                Err(e) => {
                    self.set_status(&format!("Error saving file: {e}"), StatusKind::Error);
                }
            }
        }
    }

    fn set_status(&mut self, text: &str, kind: StatusKind) {
        self.status = text.to_string();
        self.status_kind = kind;
    }
}
