use super::setting_row_d;
use crate::config::{DiscordConfig, DiscordMode};
use crate::i18n::I18n;
use eframe::egui;
use std::sync::Arc;

pub fn show(
    ui: &mut egui::Ui,
    i18n: &I18n,
    config: &mut DiscordConfig,
    connected: bool,
    needs_save: &mut bool,
    recording_keybind: &mut bool,
    keybind_capture: &mut Option<Arc<crate::hotkey::GlobalHotkeyCapture>>,
) {
    let t = |en: &str, ru: &str| i18n.pick(en, ru);
    super::section_title(ui, "Discord");
    let mut ch = false;

    let names = [
        (DiscordMode::None, t("None", "Нет")),
        (DiscordMode::Keybind, t("Keybind", "Клавиша привязки")),
        (DiscordMode::Direct, t("Direct", "Прямой")),
        (DiscordMode::Both, t("Direct + keybind", "Прямой + клавиша")),
    ];
    let cur = names.iter().find(|(m, _)| *m == config.mode).map(|(_, n)| n.clone()).unwrap_or_default();
    setting_row_d(ui, &t("Mode", "Режим"), &t("Discord integration mode", "Режим интеграции Discord"), |ui| {
        egui::ComboBox::from_id_source("discord_mode").selected_text(cur).show_ui(ui, |ui| {
            for (m, n) in &names {
                if *m == DiscordMode::Both && config.mode != DiscordMode::Both {
                    continue;
                }
                ch |= ui.selectable_value(&mut config.mode, *m, n.clone()).changed();
            }
        });
        false
    });

    let keybind_on = matches!(config.mode, DiscordMode::Keybind | DiscordMode::Both);
    let direct_on = matches!(config.mode, DiscordMode::Direct | DiscordMode::Both);

    if config.mode == DiscordMode::None {
        ui.add_space(6.0);
        ui.small(t("The headset mute button will not affect Discord.", "Кнопка мьюта на гарнитуре не будет влиять на Discord."));
    }

    if keybind_on {
        setting_row_d(ui, &t("Keybind", "Клавиша привязки"), &t("Discord mute key", "Клавиша мьюта Discord"), |ui| {
            let keybind = config.keybind.get_or_insert_with(|| "F20".to_string());
            if ui.button(if *recording_keybind { t("Cancel", "Отмена") } else { t("Record", "Записать") }).clicked() {
                if *recording_keybind {
                    if let Some(capture) = keybind_capture {
                        capture.cancel();
                    }
                    *recording_keybind = false;
                    *keybind_capture = None;
                } else {
                    *recording_keybind = true;
                    let capture = Arc::new(crate::hotkey::GlobalHotkeyCapture::new());
                    crate::hotkey::spawn_capture(capture.clone());
                    capture.start_recording();
                    *keybind_capture = Some(capture);
                }
            }
            if *recording_keybind {
                ui.colored_label(egui::Color32::from_rgb(0xd9, 0xa4, 0x00), t("Press a key combination…", "Нажмите комбинацию клавиш…"));
            } else if ui.add(egui::TextEdit::singleline(keybind).desired_width(120.0)).changed() {
                ch = true;
            }
            false
        });
        setting_row_d(ui, &t("Test the keybind", "Проверить клавишу"),
            &t("Sends the key now. In Discord: Settings → Keybinds → record, to see whether it arrives.", "Отправляет клавишу сейчас. В Discord: Настройки → Горячие клавиши → запись, чтобы увидеть, доходит ли она."), |ui| {
            if ui.button(t("Send", "Отправить")).clicked() {
                if let Err(e) = crate::input::GLOBAL_MUTE_HANDLER.send_keybind_now() {
                    log::warn!("[Discord] test key failed: {}", e);
                }
            }
            false
        });
    }

    if direct_on {
        let (status_ok, status_bad) = (t("Connected to Discord IPC", "Подключено к Discord IPC"), t("Not connected", "Не подключено"));
        setting_row_d(ui, &t("Connection", "Соединение"), &t("Discord Rich Presence and two-way mute", "Discord Rich Presence и двусторонний мьют"), |ui| {
            if connected {
                ui.colored_label(egui::Color32::from_rgb(0x20, 0xe8, 0x3a), format!("● {status_ok}"));
            } else {
                ui.colored_label(egui::Color32::from_rgb(0xe0, 0x4a, 0x4a), format!("● {status_bad}"));
            }
            false
        });
        ch |= setting_row_d(ui, &t("Application ID", "ID приложения"), &t("Discord Rich Presence application ID", "ID приложения Discord Rich Presence"),
            |ui| ui.add(egui::TextEdit::singleline(&mut config.direct.app_id).desired_width(240.0)).changed());
        ch |= setting_row_d(ui, &t("Client secret", "Client Secret"), &t("OAuth2 secret of your Discord application (needed for authorization)", "OAuth2-секрет вашего приложения Discord (нужен для авторизации)"),
            |ui| ui.add(egui::TextEdit::singleline(&mut config.direct.client_secret).password(true).desired_width(240.0)).changed());
        ch |= setting_row_d(ui, &t("Discord mute → headset", "Мьют Discord → гарнитура"),
            &t("When the microphone is muted in Discord, the headset mutes too (off — the headset mutes Discord only)", "Когда микрофон отключают в Discord, гарнитура тоже мьютится (выключено — только гарнитура мьютит Discord)"),
            |ui| ui.checkbox(&mut config.sync_from_discord, "").changed());
        ch |= setting_row_d(ui, &t("Show battery", "Показывать батарею"), &t("Battery level in the Discord status", "Заряд в статусе Discord"),
            |ui| ui.checkbox(&mut config.direct.show_battery, "").changed());
        ch |= setting_row_d(ui, &t("Show mute status", "Показывать статус мьюта"), &t("Microphone state in the Discord status", "Состояние микрофона в статусе Discord"),
            |ui| ui.checkbox(&mut config.direct.show_mute_status, "").changed());
        ui.add_space(8.0);
        ui.hyperlink_to(t("How to get an Application ID", "Как получить ID приложения"), "https://discord.com/developers/applications");
    }

    if ch {
        *needs_save = true;
    }
}
