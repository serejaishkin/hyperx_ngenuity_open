use crate::audio::debounce::DebouncedEQ;
use crate::i18n::I18n;
use eframe::egui;
use std::sync::Arc;

const BAND_FREQS: [&str; 10] = ["32", "64", "125", "250", "500", "1k", "2k", "4k", "8k", "16k"];

/// (English, Russian, bands) — same presets as the Tauri UI.
const PRESETS: &[(&str, &str, [f32; 10])] = &[
    ("Flat", "Плоский", [0.0; 10]),
    ("Bass Boost", "Усиление басов", [6.0, 4.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
    ("Bass Cut", "Ослабление басов", [-6.0, -4.0, -2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
    ("Treble Boost", "Усиление верхних", [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 2.0, 4.0, 6.0, 8.0]),
    ("Voice Chat", "Голосовой чат", [-2.0, 0.0, 2.0, 4.0, 6.0, 6.0, 4.0, 2.0, 0.0, -2.0]),
    ("Gaming", "Игры", [4.0, 3.0, 2.0, 1.0, 0.0, 0.0, 1.0, 2.0, 3.0, 4.0]),
];

pub fn show(ui: &mut egui::Ui, i18n: &I18n, bands: &mut [f32; 10], needs_save: &mut bool, apo_available: bool, debounced_eq: &Arc<DebouncedEQ>) {
    let t = |en: &str, ru: &str| i18n.pick(en, ru);
    ui.heading(t("Equalizer", "Эквалайзер"));

    if !apo_available {
        ui.colored_label(
            egui::Color32::from_rgb(0xd9, 0xa4, 0x00),
            t("System EQ is not available — install Equalizer APO / eqMac / EasyEffects", "Системный эквалайзер недоступен — установите Equalizer APO / eqMac / EasyEffects"),
        );
    }
    ui.add_space(6.0);

    // Toolbar: preset dropdown + Reset + Apply
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(t("Preset", "Пресет"));
        let current = PRESETS.iter().find(|(_, _, b)| b == bands).map(|(en, ru, _)| t(en, ru)).unwrap_or_else(|| t("Custom", "Свой"));
        egui::ComboBox::from_id_source("eq_preset").selected_text(current).show_ui(ui, |ui| {
            for (en, ru, preset) in PRESETS {
                if ui.selectable_label(bands == preset, t(en, ru)).clicked() {
                    *bands = *preset;
                    changed = true;
                }
            }
        });
        if ui.button(t("Reset", "Сбросить")).clicked() {
            *bands = [0.0; 10];
            changed = true;
        }
        if ui.add(egui::Button::new(t("Apply", "Применить")).fill(egui::Color32::from_rgb(0xd4, 0, 0))).clicked() {
            debounced_eq.schedule(*bands);
        }
    });
    ui.add_space(8.0);

    // Vertical band sliders in a row (like the Tauri `.eq-bands`)
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 18.0;
        for (freq, band) in BAND_FREQS.iter().zip(bands.iter_mut()) {
            ui.vertical(|ui| {
                ui.spacing_mut().slider_width = 170.0;
                if ui.add(egui::Slider::new(band, -12.0..=12.0).vertical().show_value(false)).changed() {
                    changed = true;
                }
                ui.label(egui::RichText::new(format!("{:+.0}", band)).size(11.0).color(egui::Color32::from_rgb(0x88, 0x88, 0x88)));
                ui.label(*freq);
            });
        }
    });
    if changed {
        *needs_save = true;
        debounced_eq.schedule(*bands);
    }

    ui.separator();

    ui.horizontal(|ui| {
        if ui.button(t("Import preset", "Импорт пресета")).clicked() {
            if let Some(path) = crate::dialog::open_import_dialog() {
                match crate::dialog::PresetFile::load(&path) {
                    Ok(preset) => {
                        *bands = preset.bands;
                        *needs_save = true;
                        debounced_eq.schedule(*bands);
                    }
                    Err(e) => {
                        log::error!("Failed to import preset: {}", e);
                    }
                }
            }
        }
        if ui.button(t("Export preset", "Экспорт пресета")).clicked() {
            let default_name = "hyperx_preset.hyperx";
            if let Some(path) = crate::dialog::open_export_dialog(default_name) {
                let preset = crate::dialog::PresetFile::new("Custom", *bands);
                if let Err(e) = preset.save(&path) {
                    log::error!("Failed to export preset: {}", e);
                }
            }
        }
        if ui.button(t("Save as preset", "Сохранить как пресет")).clicked() {
            // Save to system EQ backend
            #[cfg(target_os = "windows")]
            {
                if let Err(e) = crate::audio::windows::save_preset("custom", bands) {
                    log::error!("Failed to save preset: {}", e);
                }
            }
            #[cfg(target_os = "linux")]
            {
                if let Err(e) = crate::audio::linux::save_preset("hyperx_custom", bands) {
                    log::error!("Failed to save preset: {}", e);
                }
            }
            #[cfg(target_os = "macos")]
            {
                if let Err(e) = crate::audio::macos_eqmac::apply_eq_bands(&bands) {
                    log::error!("Failed to save preset: {}", e);
                }
            }
        }
    });
}
