use eframe::egui;
use crate::config::{Config, MuteButtonMode};
use crate::tray::icon::TrayIconConfig;
use crate::device::DeviceState;
use crate::audio::debounce::DebouncedEQ;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub mod eq_tab;
pub mod discord_tab;
pub mod dock;
#[cfg(windows)]
pub mod taskbar_dock;

pub struct HyperXApp {
    pub config: Config,
    pub device_state: DeviceState,
    pub discord_connected: bool,
    pub selected_tab: Tab,
    pub eq_bands: [f32; 10],
    pub needs_save: bool,
    pub apo_available: bool,
    pub tray_tx: Option<std::sync::mpsc::Sender<crate::tray::TrayCommand>>,
    pub tray: Option<crate::tray::PlatformTray>,
    pub debounced_eq: Arc<DebouncedEQ>,
    pub recording_keybind: bool,
    pub device_rx: Option<std::sync::mpsc::Receiver<crate::DeviceEvent>>,
    pub device_cmd_tx: Option<std::sync::mpsc::Sender<crate::DeviceCommand>>,
    pub keybind_capture: Option<Arc<crate::hotkey::GlobalHotkeyCapture>>,
    pub last_battery_warning: Option<u8>,
    pub tray_rx: Option<std::sync::mpsc::Receiver<crate::tray::TrayCommand>>,
    pub volume: f32,
    pub mic_volume: f32,
    pub should_exit: bool,
    pub window_hidden: bool,
    pub last_volume_check: Instant,
    pub show_discord_panel: bool,
    pub selected_settings_tab: SettingsTab,
    pub tray_icon_config: TrayIconConfig,
    pub tray_cfg_pending_save: bool,
    pub i18n: crate::i18n::I18n,
    pub audio_devices: Vec<crate::system_audio::AudioDevice>,
    pub audio_inputs: Vec<crate::system_audio::AudioDevice>,
    pub selected_audio_device: Option<String>,
    pub last_audio_device_scan: Instant,
    #[cfg(target_os = "windows")]
    pub volume_controller: Option<crate::platform::windows::volume::WindowsVolume>,
    #[cfg(target_os = "linux")]
    pub volume_controller: Option<crate::platform::linux::volume::LinuxVolume>,
    #[cfg(target_os = "macos")]
    pub volume_controller: Option<crate::platform::macos::volume::MacOSVolume>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingsTab {
    Headset,
    Voice,
    TrayIcon,
    Discord,
    Audio,
    Debug,
    Interface,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[derive(Default)]
pub enum Tab {
    #[default]
    Dashboard,
    Equalizer,
    Settings,
}


impl HyperXApp {
    pub fn new(config: Config, device_state: DeviceState, apo_available: bool, debounced_eq: Arc<DebouncedEQ>, i18n: crate::i18n::I18n) -> Self {
        let eq_bands = config.audio.eq_bands;
        let saved_audio_device = config.audio.fallback_output_device.clone();
        Self {
            config,
            device_state,
            discord_connected: false,
            selected_tab: Tab::Dashboard,
            eq_bands,
            needs_save: false,
            apo_available,
            tray_tx: None,
            tray: None,
            debounced_eq,
            recording_keybind: false,
            device_rx: None,
            device_cmd_tx: None,
            keybind_capture: None,
            last_battery_warning: None,
            tray_rx: None,
            volume: 50.0,
            mic_volume: 50.0,
            should_exit: false,
            window_hidden: false,
            last_volume_check: Instant::now(),
            show_discord_panel: false,
            selected_settings_tab: SettingsTab::Headset,
            tray_icon_config: TrayIconConfig::load_or_create(),
            tray_cfg_pending_save: false,
            i18n,
            audio_devices: Vec::new(),
            audio_inputs: Vec::new(),
            selected_audio_device: saved_audio_device,
            last_audio_device_scan: Instant::now() - Duration::from_secs(5),
            #[cfg(target_os = "windows")]
            volume_controller: Some(crate::platform::windows::volume::WindowsVolume::new()),
            #[cfg(target_os = "linux")]
            volume_controller: Some(crate::platform::linux::volume::LinuxVolume::new()),
            #[cfg(target_os = "macos")]
            volume_controller: Some(crate::platform::macos::volume::MacOSVolume::new()),
        }
    }

    pub fn with_tray(mut self, tx: std::sync::mpsc::Sender<crate::tray::TrayCommand>) -> Self {
        self.tray_tx = Some(tx);
        self
    }

    pub fn with_tray_backend(mut self, tray: crate::tray::PlatformTray) -> Self {
        self.tray = Some(tray);
        self
    }

    pub fn with_device_receiver(mut self, rx: std::sync::mpsc::Receiver<crate::DeviceEvent>) -> Self {
        self.device_rx = Some(rx);
        self
    }

    pub fn with_device_command_sender(mut self, tx: std::sync::mpsc::Sender<crate::DeviceCommand>) -> Self {
        self.device_cmd_tx = Some(tx);
        self
    }

    pub fn with_tray_receiver(mut self, rx: std::sync::mpsc::Receiver<crate::tray::TrayCommand>) -> Self {
        self.tray_rx = Some(rx);
        self
    }

    fn save_config(&mut self) {
        self.config.audio.eq_bands = self.eq_bands;
        log::info!("[GUI] Saving config...");
        if let Err(e) = self.config.save() {
            log::error!("[GUI] Failed to save config: {}", e);
        } else {
            log::info!("[GUI] Config saved successfully");
            self.needs_save = false;
        }
    }

    fn apply_eq_preset(&mut self, preset: &str) {
        log::info!("[GUI] EQ preset applied: {}", preset);
        self.eq_bands = match preset {
            "Flat" => [0.0; 10],
            "Bass Boost" => [6.0, 4.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            "Bass Cut" => [-6.0, -4.0, -2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            "Treble Boost" => [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 2.0, 4.0, 6.0, 8.0],
            "Voice Chat" => [-2.0, 0.0, 2.0, 4.0, 6.0, 6.0, 4.0, 2.0, 0.0, -2.0],
            "Gaming" => [4.0, 3.0, 2.0, 1.0, 0.0, 0.0, 1.0, 2.0, 3.0, 4.0],
            _ => self.eq_bands,
        };
        self.needs_save = true;
    }
}

impl eframe::App for HyperXApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(100));
        self.show_dock(ctx);

        apply_tauri_theme(ctx);

        if let Some(tray) = &mut self.tray {
            tray.poll();
        }

        // Tray commands
        if let Some(rx) = &self.tray_rx {
            while let Ok(cmd) = rx.try_recv() {
                match cmd {
                    crate::tray::TrayCommand::ToggleMute => {
                        if let Some(tx) = &self.device_cmd_tx {
                            let _ = tx.send(crate::DeviceCommand::ToggleMute);
                        }
                    }
                    crate::tray::TrayCommand::ShowWindow => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                        self.window_hidden = false;
                        ctx.request_repaint();
                    }
                    crate::tray::TrayCommand::Quit => {
                        std::process::exit(0);
                    }
                    crate::tray::TrayCommand::RefreshBattery => {}
                }
            }
        }

        if ctx.input(|i| i.viewport().close_requested()) {
            std::process::exit(0);
        }

        if !self.window_hidden
            && ctx.input(|i| i.viewport().minimized) == Some(true) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                self.window_hidden = true;
            }

        if let Some(capture) = &self.keybind_capture {
            if let Some(combo) = capture.poll_result() {
                let key_str = combo.display.clone();
                self.config.discord.keybind = Some(key_str.clone());
                crate::input::GLOBAL_MUTE_HANDLER.set_keybind(Some(key_str));
                self.recording_keybind = false;
                self.keybind_capture = None;
                self.needs_save = true;
            }
        }

        if let Some(keybind) = &self.config.discord.keybind {
            crate::input::GLOBAL_MUTE_HANDLER.set_keybind(Some(keybind.clone()));
        }
        crate::input::GLOBAL_MUTE_HANDLER.set_keybind_enabled(
            self.config.discord.mode.uses_keybind(),
        );
        crate::input::GLOBAL_MUTE_HANDLER.set_discord_direct(self.config.discord.mode.uses_direct());
        crate::input::GLOBAL_MUTE_HANDLER.set_discord_to_headset(self.config.discord.sync_from_discord);
        crate::input::GLOBAL_MUTE_HANDLER
            .set_sidetone_play_pause(self.config.device.sidetone_play_pause);

        // Device events
        if let Some(rx) = &self.device_rx {
            while let Ok(event) = rx.try_recv() {
                match event {
                    crate::DeviceEvent::StateChanged(state) => {
                        self.device_state = state.clone();
                        if let Some(tray) = &mut self.tray {
                            tray.update_connected(state.connected);
                            tray.update_battery(state.battery_percent, state.charging);
                            tray.update_mute(state.muted);
                        }
                    }
                    crate::DeviceEvent::Connected => {
                        self.device_state.connected = true;
                        if let Some(tray) = &mut self.tray {
                            tray.update_connected(true);
                        }
                    }
                    crate::DeviceEvent::Disconnected => {
                        self.device_state.connected = false;
                        if let Some(tray) = &mut self.tray {
                            tray.update_connected(false);
                        }
                    }
                    crate::DeviceEvent::BatteryLow(percent) => {
                        if self.last_battery_warning != Some(percent) {
                            self.last_battery_warning = Some(percent);
                        }
                    }
                }
            }
        }

        // Volume polling (master + mic) - all platforms
        if self.last_volume_check.elapsed() > Duration::from_millis(200) {
            if let Some(ref controller) = self.volume_controller {
                if let Some(vol) = controller.get_master_volume() {
                    self.volume = vol;
                }
                if let Some(mic_vol) = controller.get_microphone_volume() {
                    self.mic_volume = mic_vol;
                }
            }
            self.last_volume_check = Instant::now();
        }

        if self.config.compact_mode {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize([260.0, 360.0].into()));
            ctx.send_viewport_cmd(egui::ViewportCommand::Resizable(false));
            self.show_compact_ui(ctx);
        } else {
            ctx.send_viewport_cmd(egui::ViewportCommand::Resizable(true));

        // === TOP PANEL ===
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 18.0;
                for (tab, key) in [(Tab::Dashboard, "Dashboard"), (Tab::Equalizer, "Equalizer"), (Tab::Settings, "Settings")] {
                    if underline_tab(ui, self.selected_tab == tab, &self.i18n.t(key)).clicked() {
                        self.selected_tab = tab;
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("⛶").clicked() {
                        self.config.compact_mode = !self.config.compact_mode;
                        self.needs_save = true;
                        if !self.config.compact_mode {
                            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize([980.0, 600.0].into()));
                        }
                    }
                    ui.separator();
                    if self.device_state.connected {
                        let icon = if self.device_state.charging { "[CHG]" } else { "[BAT]" };
                        ui.label(format!("{} {}%", icon, self.device_state.battery_percent));
                        ui.colored_label(egui::Color32::from_rgb(0x20, 0xe8, 0x3a), "ON");
                    } else {
                        ui.colored_label(egui::Color32::RED, &self.i18n.t("OFF - No connection"));
                    }
                });
            });
        });

        // === CENTER ===
        egui::CentralPanel::default().frame(egui::Frame::none().fill(egui::Color32::from_rgb(0x18, 0x18, 0x18)).inner_margin(egui::Margin::symmetric(18.0, 14.0))).show(ctx, |ui| {
            match self.selected_tab {
                Tab::Dashboard => self.show_dashboard(ui),
                Tab::Equalizer => self.show_equalizer(ui),
                Tab::Settings => self.show_settings(ui),
            }
        });

        if self.needs_save {
            egui::TopBottomPanel::bottom("save_bar").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::YELLOW, self.i18n.t("(!) Unsaved changes"));
                    if ui.button("Save").clicked() { self.save_config(); }
                });
            });
        }
        }
    }
}

impl HyperXApp {
    fn show_compact_ui(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Compact layout is intentionally kept within the fixed 260x300 window.
            ui.vertical_centered(|ui| {
                ui.add_space(3.0);

                let icon = if self.device_state.charging { "⚡" } else { "🔋" };
                let color = if self.device_state.battery_percent > 30 { egui::Color32::from_rgb(0x20, 0xe8, 0x3a) }
                    else if self.device_state.battery_percent > 15 { egui::Color32::YELLOW }
                    else { egui::Color32::RED };
                ui.colored_label(color, format!("{} {}%", icon, self.device_state.battery_percent));

                // --- System output device ---
                // Put the selector near the top so it cannot be pushed below the
                // visible area by the volume/microphone controls.
                if self.audio_devices.is_empty() && self.last_audio_device_scan.elapsed() >= Duration::from_secs(1) {
                    self.last_audio_device_scan = Instant::now();
                    match crate::system_audio::get_audio_devices() {
                        Ok(devices) => {
                            self.audio_inputs = devices.iter().filter(|d| d.is_input).cloned().collect();
                            self.audio_devices = devices.into_iter().filter(|d| d.is_output).collect();
                            log::info!("[GUI] Compact audio scan: {} output device(s)", self.audio_devices.len());
                        }
                        Err(e) => log::error!("[GUI] Compact audio scan failed: {}", e),
                    }

                    if self.selected_audio_device.is_none() {
                        if let Ok(default_id) = crate::system_audio::get_default_output_device() {
                            if self.audio_devices.iter().any(|d| d.id == default_id) {
                                self.selected_audio_device = Some(default_id);
                            }
                        }
                    }
                }

                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.label("🔊");
                    if self.audio_devices.is_empty() {
                        ui.add_enabled(false, egui::Button::new(self.i18n.pick("No audio devices found", "Аудиоустройства не найдены")));
                    } else {
                        let selected_name = self.selected_audio_device.as_ref()
                            .and_then(|id| self.audio_devices.iter().find(|d| &d.id == id))
                            .map(|d| d.name.clone())
                            .unwrap_or_else(|| self.audio_devices[0].name.clone());

                                egui::ComboBox::from_id_salt("compact_audio_output")
                            .selected_text(selected_name)
                            .width(185.0)
                            .show_ui(ui, |ui| {
                                for device in self.audio_devices.clone() {
                                    let selected = self.selected_audio_device.as_ref() == Some(&device.id);
                                    if ui.selectable_label(selected, &device.name).clicked() {
                                        match crate::system_audio::set_default_output_device(&device.id) {
                                            Ok(()) => {
                                                self.selected_audio_device = Some(device.id.clone());
                                                log::info!("[GUI] Default output changed to: {}", device.name);
                                            }
                                            Err(e) => {
                                                log::error!("[GUI] Failed to change output '{}': {}", device.name, e);
                                            }
                                        }
                                        ui.close_menu();
                                    }
                                }
                            });
                    }

                    if ui.small_button("↻").on_hover_text(self.i18n.pick("Refresh audio devices", "Обновить аудиоустройства")).clicked() {
                        self.audio_devices.clear();
                        self.selected_audio_device = None;
                        self.last_audio_device_scan = Instant::now() - Duration::from_secs(1);
                    }
                });

                // Fallback (reserve) devices: remembered and saved automatically on change.
                {
                    let mut changed = false;
                    let out_name = self.config.audio.fallback_output_device.as_ref()
                        .and_then(|id| self.audio_devices.iter().find(|d| &d.id == id))
                        .map(|d| d.name.clone())
                        .unwrap_or_else(|| self.i18n.pick("— not set —", "— не задан —"));
                    ui.horizontal(|ui| {
                        ui.label("↩🔊");
                        egui::ComboBox::from_id_salt("compact_fallback_output")
                            .selected_text(out_name)
                            .width(185.0)
                            .show_ui(ui, |ui| {
                                for d in self.audio_devices.clone() {
                                    let sel = self.config.audio.fallback_output_device.as_ref() == Some(&d.id);
                                    if ui.selectable_label(sel, &d.name).clicked() {
                                        self.config.audio.fallback_output_device = Some(d.id.clone());
                                        changed = true;
                                    }
                                }
                            });
                    });
                    let in_name = self.config.audio.fallback_input_device.as_ref()
                        .and_then(|id| self.audio_inputs.iter().find(|d| &d.id == id))
                        .map(|d| d.name.clone())
                        .unwrap_or_else(|| self.i18n.pick("— not set —", "— не задан —"));
                    ui.horizontal(|ui| {
                        ui.label("↩🎙");
                        egui::ComboBox::from_id_salt("compact_fallback_input")
                            .selected_text(in_name)
                            .width(185.0)
                            .show_ui(ui, |ui| {
                                for d in self.audio_inputs.clone() {
                                    let sel = self.config.audio.fallback_input_device.as_ref() == Some(&d.id);
                                    if ui.selectable_label(sel, &d.name).clicked() {
                                        self.config.audio.fallback_input_device = Some(d.id.clone());
                                        changed = true;
                                    }
                                }
                            });
                    });
                    if changed {
                        self.config.audio.auto_switch_audio = true;
                        self.needs_save = true;
                        log::info!("[GUI] Fallback devices saved: out={:?} in={:?}",
                            self.config.audio.fallback_output_device, self.config.audio.fallback_input_device);
                    }
                }

                ui.add_space(3.0);
                let mic_icon = if self.device_state.muted { "🔇" } else { "🎙️" };
                let mic_text = if self.device_state.muted { self.i18n.t("MUTE") } else { self.i18n.t("MIC ON") };
                ui.label(format!("{} {}", mic_icon, mic_text));

                ui.add_space(2.0);
                ui.label(self.i18n.t("VOL"));
                let mut vol = self.volume;
                ui.add(egui::Slider::new(&mut vol, 0.0..=100.0).show_value(true).text(""));
                if vol != self.volume {
                    self.volume = vol;
                    if let Some(ref controller) = self.volume_controller {
                        controller.set_master_volume(vol);
                    }
                }

                ui.add_space(2.0);
                ui.label(self.i18n.t("MIC"));
                let mut mic_vol = self.mic_volume;
                ui.add(egui::Slider::new(&mut mic_vol, 0.0..=100.0).show_value(true).text(""));
                if mic_vol != self.mic_volume {
                    self.mic_volume = mic_vol;
                    if let Some(ref controller) = self.volume_controller {
                        controller.set_microphone_volume(mic_vol);
                    }
                }

                ui.add_space(2.0);
                if ui.button(format!("⛶ {}", self.i18n.t("Expand"))).clicked() {
                    self.config.compact_mode = false;
                    self.needs_save = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize([980.0, 600.0].into()));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Resizable(true));
                }
            });
        });
    }

    /// VOL / MIC sliders + mic button + sidetone + play (same layout as the Tauri dashboard).
    fn show_controls(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        ui.columns(2, |cols| {
            cols[0].horizontal(|ui| {
                ui.label(egui::RichText::new(self.i18n.t("VOL")).color(egui::Color32::from_rgb(0xa5, 0xa5, 0xa5)));
                let mut vol = self.volume;
                ui.add(egui::Slider::new(&mut vol, 0.0..=100.0).show_value(false).trailing_fill(false));
                ui.label(format!("{:.0}%", self.volume));
                if vol != self.volume {
                    self.volume = vol;
                    if let Some(ref c) = self.volume_controller { c.set_master_volume(vol); }
                }
            });
            cols[1].horizontal(|ui| {
                ui.label(egui::RichText::new(self.i18n.t("MIC")).color(egui::Color32::from_rgb(0xa5, 0xa5, 0xa5)));
                let mut mv = self.mic_volume;
                ui.add(egui::Slider::new(&mut mv, 0.0..=100.0).show_value(false).trailing_fill(false));
                ui.label(format!("{:.0}%", self.mic_volume));
                if mv != self.mic_volume {
                    self.mic_volume = mv;
                    if let Some(ref c) = self.volume_controller { c.set_microphone_volume(mv); }
                }
            });
        });
        ui.horizontal(|ui| {
            let muted = self.volume_controller.as_ref().and_then(|c| c.get_microphone_mute()).unwrap_or(self.device_state.muted);
            let label = if muted { self.i18n.t("MUTE") } else { self.i18n.t("MIC ON") };
            let active = muted;
            let mut b = egui::Button::new(label).min_size(egui::vec2(70.0, 30.0));
            if active { b = b.fill(egui::Color32::from_rgb(0xd4, 0, 0)); }
            if ui.add(b).clicked() {
                if let Some(ref c) = self.volume_controller { c.set_microphone_mute(!muted); }
                else if let Some(tx) = &self.device_cmd_tx { let _ = tx.send(crate::DeviceCommand::ToggleMute); }
            }
            let mut st = self.device_state.sidetone;
            if ui.checkbox(&mut st, self.i18n.t("Sidetone")).changed() {
                if let Some(tx) = &self.device_cmd_tx { let _ = tx.send(crate::DeviceCommand::SetSidetone(st)); }
                self.device_state.sidetone = st;
            }
            if ui.add(egui::Button::new("PLAY").min_size(egui::vec2(64.0, 30.0))).clicked() {
                crate::input::GLOBAL_MUTE_HANDLER.do_media_play_pause();
            }
        });
        ui.add_space(6.0);
    }

    fn show_dashboard(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.i18n.t("Headset Status"));
        ui.separator();

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(self.i18n.t("Battery") + ":");
                if self.device_state.charging {
                    ui.colored_label(egui::Color32::YELLOW, "Charging");
                }
                ui.label(format!("{}%", self.device_state.battery_percent));
                let battery = self.device_state.battery_percent as f32 / 100.0;
                let color = if self.device_state.charging { egui::Color32::from_rgb(0x20, 0xe8, 0x3a) }
                    else if self.device_state.battery_percent <= 20 { egui::Color32::from_rgb(0xe0, 0x4a, 0x4a) }
                    else { egui::Color32::from_rgb(0xd9, 0xa4, 0x00) };
                let (rect, _) = ui.allocate_exact_size(egui::vec2(300.0, 18.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 9.0, egui::Color32::from_rgb(5, 5, 5));
                if battery > 0.0 {
                    let mut fill = rect;
                    fill.set_width((rect.width() * battery).max(18.0));
                    ui.painter().rect_filled(fill, 9.0, color);
                }
            });

            ui.separator();

            ui.vertical(|ui| {
                ui.label(self.i18n.t("Microphone") + ":");
                if self.device_state.muted {
                    ui.colored_label(egui::Color32::from_rgb(0xe0, 0x4a, 0x4a), self.i18n.t("Muted"));
                } else {
                    ui.colored_label(egui::Color32::from_rgb(0x20, 0xe8, 0x3a), self.i18n.t("Active"));
                }
                ui.add_space(10.0);
                ui.label(self.i18n.t("Signal:"));
                ui.label(format!("{} dBm", self.device_state.signal_dbm));
            });
        });

        ui.add_space(6.0);
        ui.separator();
        self.show_controls(ui);
        ui.separator();
        ui.heading(self.i18n.t("Quick Actions"));
        ui.horizontal(|ui| {
            if ui.button(if self.device_state.muted { self.i18n.t("Unmute Mic") } else { self.i18n.t("Mute Mic") }).clicked() {
                log::info!("[GUI] Dashboard: ToggleMute clicked");
                if let Some(tx) = &self.device_cmd_tx { let _ = tx.send(crate::DeviceCommand::ToggleMute); }
            }
            if ui.button(self.i18n.t("Check Battery (Voice)")).clicked() {
                log::info!("[GUI] Dashboard: Check Battery voice clicked");
                crate::audio::voice::play(crate::audio::voice::VoiceEvent::Battery(self.device_state.battery_percent));
            }
        });
    }

    fn show_equalizer(&mut self, ui: &mut egui::Ui) {
        eq_tab::show(ui, &self.i18n, &mut self.eq_bands, &mut self.needs_save, self.apo_available, &self.debounced_eq);
    }

    fn show_settings(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            for (tab, key) in [
                (SettingsTab::Headset, "Headset"), (SettingsTab::Voice, "Voice"), (SettingsTab::TrayIcon, "Tray Icon"),
                (SettingsTab::Discord, "Discord"), (SettingsTab::Audio, "Audio"), (SettingsTab::Debug, "Debug"),
                (SettingsTab::Interface, "Interface"),
            ] {
                let sel = self.selected_settings_tab == tab;
                let txt = egui::RichText::new(self.i18n.t(key)).color(if sel { egui::Color32::WHITE } else { egui::Color32::from_rgb(0x88, 0x88, 0x88) });
                let btn = egui::Button::new(txt)
                    .fill(if sel { egui::Color32::from_rgb(0x22, 0x22, 0x22) } else { egui::Color32::from_rgb(0x1b, 0x1b, 0x1b) })
                    .stroke(if sel { egui::Stroke::new(1.0, egui::Color32::from_rgb(0x3b, 0x3b, 0x3b)) } else { egui::Stroke::NONE })
                    .rounding(0.0)
                    .min_size(egui::vec2(0.0, 34.0));
                if ui.add(btn).clicked() { self.selected_settings_tab = tab; }
            }
        });
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            match self.selected_settings_tab {
                SettingsTab::Headset => self.show_settings_headset(ui),
                SettingsTab::Voice => self.show_settings_voice(ui),
                SettingsTab::TrayIcon => self.show_settings_tray_icon(ui),
                SettingsTab::Discord => self.show_settings_discord(ui),
                SettingsTab::Audio => self.show_settings_audio(ui),
                SettingsTab::Debug => self.show_settings_debug(ui),
                SettingsTab::Interface => self.show_settings_interface(ui),
            }
        });
    }

    fn show_settings_headset(&mut self, ui: &mut egui::Ui) {
        use crate::input::GLOBAL_MUTE_HANDLER;
        let t = |en: &str, ru: &str| self.i18n.pick(en, ru);
        let (min_u, ms_u) = (t("min", "мин"), t("ms", "мс"));
        section_title(ui, &t("Headset", "Гарнитура"));

        if setting_row_d(ui, &t("App enabled", "Приложение включено"), &t("Enable headset control", "Включить управление гарнитурой"),
            |ui| ui.checkbox(&mut self.config.enabled, "").changed()) {
            self.needs_save = true;
        }
        if setting_row_d(ui, &t("Sidetone", "Сайдтон"), &t("Hear your own microphone in the headset", "Слышать свой микрофон в гарнитуре"),
            |ui| ui.checkbox(&mut self.config.device.sidetone, "").changed()) {
            self.needs_save = true;
            self.device_state.sidetone = self.config.device.sidetone;
            if let Some(tx) = &self.device_cmd_tx { let _ = tx.send(crate::DeviceCommand::SetSidetone(self.config.device.sidetone)); }
        }
        if setting_row_d(ui, &t("Voice prompts", "Голосовые подсказки"), &t("Remember the headset's voice prompt setting", "Запомнить настройку голосовых подсказок гарнитуры"),
            |ui| ui.checkbox(&mut self.config.device.voice_prompts, "").changed()) {
            self.needs_save = true;
            if let Some(tx) = &self.device_cmd_tx { let _ = tx.send(crate::DeviceCommand::SetVoicePrompts(self.config.device.voice_prompts)); }
        }
        if setting_row_d(ui, &t("Auto shutdown", "Авто-выключение"), &t("0 — timer off", "0 — таймер выключен"), |ui| {
            let r = ui.add(egui::DragValue::new(&mut self.config.device.auto_shutdown_minutes).range(0..=60).suffix(format!(" {min_u}")));
            r.drag_stopped() || (r.lost_focus() && r.changed())
        }) {
            self.needs_save = true;
            if let Some(tx) = &self.device_cmd_tx { let _ = tx.send(crate::DeviceCommand::SetAutoShutdown(self.config.device.auto_shutdown_minutes)); }
        }

        section_title(ui, &t("Mute button", "Кнопка мьюта"));
        let prev_mode = self.config.input.mute_button_mode;
        let modes = [
            (MuteButtonMode::Standard, t("Standard (MicMute)", "Стандартный (Микр.мьют)")),
            (MuteButtonMode::MediaPlayPause, t("Play/Pause", "Воспр./Пауза")),
            (MuteButtonMode::SmartDouble, t("Smart: single = mute / double = play/pause", "Умный: одно нажатие — мьют / двойное — воспр./пауза")),
            (MuteButtonMode::SmartHold, t("Smart: short = play/pause / hold = mute", "Умный: короткое — воспр./пауза / удержание — мьют")),
            (MuteButtonMode::HoldPlayPause, t("Smart: short = mute / hold = play/pause", "Умный: короткое — мьют / удержание — воспр./пауза")),
        ];
        let cur_name = modes.iter().find(|(m, _)| *m == prev_mode).map(|(_, n)| n.clone()).unwrap_or_default();
        setting_row_d(ui, &t("Mode", "Режим"), &t("Headset button behaviour", "Поведение кнопки гарнитуры"), |ui| {
            egui::ComboBox::from_id_source("mute_mode").width(380.0).selected_text(cur_name).show_ui(ui, |ui| {
                for (m, name) in &modes {
                    ui.selectable_value(&mut self.config.input.mute_button_mode, *m, name.clone());
                }
            });
            false
        });
        if self.config.input.mute_button_mode != prev_mode {
            self.needs_save = true;
            GLOBAL_MUTE_HANDLER.set_mode(match self.config.input.mute_button_mode {
                MuteButtonMode::Standard => crate::input::MuteButtonMode::Standard,
                MuteButtonMode::MediaPlayPause => crate::input::MuteButtonMode::MediaPlayPause,
                MuteButtonMode::SmartDouble => crate::input::MuteButtonMode::SmartDouble,
                MuteButtonMode::SmartHold => crate::input::MuteButtonMode::SmartHold,
                MuteButtonMode::HoldPlayPause => crate::input::MuteButtonMode::HoldPlayPause,
            });
        }
        if setting_row_d(ui, &t("Sidetone button = Play/Pause", "Кнопка сайдтона = Воспр./Пауза"),
            &t("The headset's sidetone button pauses music while the sidetone state stays unchanged.", "Кнопка сайдтона на гарнитуре ставит музыку на паузу, а состояние сайдтона остаётся прежним."),
            |ui| ui.checkbox(&mut self.config.device.sidetone_play_pause, "").changed()) {
            self.needs_save = true;
            GLOBAL_MUTE_HANDLER.set_sidetone_play_pause(self.config.device.sidetone_play_pause);
        }
        if setting_row_d(ui, &t("Double-tap delay", "Задержка двойного нажатия"), &t("Smart double-click interval", "Интервал умного двойного клика"),
            |ui| ui.add(egui::DragValue::new(&mut self.config.double_tap_ms).range(100..=1000).suffix(format!(" {ms_u}"))).changed()) {
            self.needs_save = true;
        }
    }
    fn show_settings_voice(&mut self, ui: &mut egui::Ui) {
        let t = |en: &str, ru: &str| self.i18n.pick(en, ru);
        section_title(ui, &t("Built-in WAV voices", "Встроенные WAV-голоса"));
        ui.label(egui::RichText::new(t(
            "The project contains all battery voice prompts from bat_000.wav to bat_100.wav (0% to 100%), as well as WAV files for charging, full charge and low battery.",
            "Проект содержит все голосовые подсказки батареи от bat_000.wav до bat_100.wav (0% до 100%), а также WAV-файлы зарядки, полной зарядки и низкого заряда батареи.",
        )).size(12.0).color(egui::Color32::from_rgb(0x88, 0x88, 0x88)));
        let mut ch = false;
        ch |= setting_row_d(ui, &t("Enable voice", "Включить голос"), &t("Enable voice notifications about battery and status", "Включить голосовые уведомления о батарее/статусе"),
            |ui| ui.checkbox(&mut self.config.voice.enabled, "").changed());
        ch |= setting_row_d(ui, &t("Battery low", "Батарея разряжена"), &t("Play low_battery.wav", "Воспроизвести low_battery.wav"),
            |ui| ui.checkbox(&mut self.config.voice.on_battery_low, "").changed());
        ch |= setting_row_d(ui, &t("Say the percentage", "Называть процент"), &t("After \"battery low\" say the remaining percentage", "После фразы «батарея разряжена» произносить остаток в процентах"),
            |ui| ui.checkbox(&mut self.config.voice.low_say_percent, "").changed());
        let lvl_title = t("Warn at levels", "Предупреждать на уровнях");
        let lvl_desc = t("Each level fires once per discharge", "Каждый уровень срабатывает один раз за разряд");
        ch |= setting_row_d(ui, &lvl_title, &lvl_desc, |ui| {
            let mut c = false;
            for lvl in [10u8, 20, 30, 40, 50] {
                let mut on = self.config.voice.low_levels.contains(&lvl);
                if ui.checkbox(&mut on, format!("{lvl}%")).changed() {
                    if on { self.config.voice.low_levels.push(lvl); } else { self.config.voice.low_levels.retain(|l| *l != lvl); }
                    self.config.voice.low_levels.sort_unstable_by(|a, b| b.cmp(a));
                    c = true;
                }
            }
            c
        });
        let off = t("Off", "Выкл");
        ch |= setting_row_d(ui, &t("Repeat at the minimum", "Дублировать на минимуме"), &t("At this level and below the warning sounds twice", "На этом уровне и ниже предупреждение звучит дважды"), |ui| {
            let cur = self.config.voice.low_critical_percent;
            let mut c = false;
            egui::ComboBox::from_id_source("low_crit")
                .selected_text(if cur == 0 { off.clone() } else { format!("{cur}%") })
                .show_ui(ui, |ui| {
                    for v in [0u8, 30, 20, 10, 5] {
                        let label = if v == 0 { off.clone() } else { format!("{v}%") };
                        if ui.selectable_value(&mut self.config.voice.low_critical_percent, v, label).changed() { c = true; }
                    }
                });
            c
        });
        ch |= setting_row_d(ui, &t("Charging", "Зарядка"), &t("Play charging.wav", "Воспроизвести charging.wav"),
            |ui| ui.checkbox(&mut self.config.voice.on_charging, "").changed());
        ch |= setting_row_d(ui, &t("Full charge", "Полная зарядка"), &t("Play full_charge.wav", "Воспроизвести full_charge.wav"),
            |ui| ui.checkbox(&mut self.config.voice.on_full_charge, "").changed());
        ch |= setting_row_d(ui, &t("Connect notification", "Уведомление о подключении"), &t("Only available voice events are used", "Используются только доступные голосовые события"),
            |ui| ui.checkbox(&mut self.config.voice.on_connected, "").changed());
        ch |= setting_row_d(ui, &t("Disconnect notification", "Уведомление об отключении"), &t("Only available voice events are used", "Используются только доступные голосовые события"),
            |ui| ui.checkbox(&mut self.config.voice.on_disconnected, "").changed());
        ch |= setting_row_d(ui, &t("Button check", "Проверка кнопки"), &t("Allow the battery check (voice)", "Разрешить проверку батареи (голос)"),
            |ui| ui.checkbox(&mut self.config.voice.on_button_check, "").changed());
        ch |= setting_row_d(ui, &t("Battery clip mode", "Режим клипа батареи"), &t("The nearest built-in clip is used", "Используется ближайший встроенный клип"),
            |ui| ui.checkbox(&mut self.config.voice.exact_percent, "").changed());
        if ch {
            self.needs_save = true;
            crate::audio::voice::update_config(self.config.voice.clone());
        }
    }

    fn show_settings_tray_icon(&mut self, ui: &mut egui::Ui) {
        use crate::tray::icon::{generate_tray_icon_rgba, TrayIconMode};
        let t = |en: &str, ru: &str| self.i18n.pick(en, ru);
        let tray_cfg_before = self.tray_icon_config.clone();

        // --- Battery indicator (floating / on taskbar) ---
        section_title(ui, &t("Battery indicator (headphones + %)", "Индикатор заряда (наушники и %)"));
        let (t_off, t_float, t_task) = (t("Off", "Выключен"), t("Floating", "Плавающий"), t("On taskbar (Windows)", "На панели задач (Windows)"));
        let mut sel = if !self.config.taskbar_widget { "off" } else if cfg!(windows) && self.config.dock_mode == "taskbar" { "taskbar" } else { "floating" };
        let before = sel;
        setting_row_d(ui, &t("Indicator placement", "Расположение индикатора"),
            &t("Floating — in the screen corner, drag anywhere. On taskbar — next to the tray (Windows).", "Плавающий — в углу экрана, перетаскивается куда угодно. На панели задач — рядом с треем (Windows)."), |ui| {
            egui::ComboBox::from_id_source("dock_mode")
                .selected_text(match sel { "off" => t_off.as_str(), "taskbar" => t_task.as_str(), _ => t_float.as_str() })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut sel, "off", &t_off);
                    ui.selectable_value(&mut sel, "floating", &t_float);
                    #[cfg(windows)]
                    ui.selectable_value(&mut sel, "taskbar", &t_task);
                });
            false
        });
        if sel != before {
            self.config.taskbar_widget = sel != "off";
            if sel != "off" { self.config.dock_mode = sel.to_string(); }
            self.needs_save = true;
        }
        let (s_icon, s_pct, s_dig) = (t("Headphones + %", "Наушники + %"), t("Percent only", "Только %"), t("Digits only", "Только цифры"));
        let mut st = self.config.dock_style.clone();
        setting_row_d(ui, &t("Indicator style", "Стиль индикатора"), &t("Right-click the indicator for its menu", "Правый клик по индикатору открывает меню"), |ui| {
            egui::ComboBox::from_id_source("dock_style")
                .selected_text(match st.as_str() { "digits" => s_dig.as_str(), "percent" => s_pct.as_str(), _ => s_icon.as_str() })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut st, "icon".to_string(), &s_icon);
                    ui.selectable_value(&mut st, "percent".to_string(), &s_pct);
                    ui.selectable_value(&mut st, "digits".to_string(), &s_dig);
                });
            false
        });
        if st != self.config.dock_style { self.config.dock_style = st; self.needs_save = true; }

        // --- Tray icon ---
        section_title(ui, &t("Tray icon — preview", "Иконка в трее — Превью"));
        ui.horizontal(|ui| {
            let (rgba, w, h) = generate_tray_icon_rgba(&self.tray_icon_config, 75, false);
            let image = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
            let texture = ui.ctx().load_texture("preview_75", image, egui::TextureOptions::NEAREST);
            ui.add(egui::Image::new(&texture).fit_to_exact_size([64.0, 64.0].into()));
            let (rgba2, w2, h2) = generate_tray_icon_rgba(&self.tray_icon_config, 42, true);
            let image2 = egui::ColorImage::from_rgba_unmultiplied([w2 as usize, h2 as usize], &rgba2);
            let texture2 = ui.ctx().load_texture("preview_chg", image2, egui::TextureOptions::NEAREST);
            ui.add(egui::Image::new(&texture2).fit_to_exact_size([64.0, 64.0].into()));
        });
        let (m_big, m_bat) = (t("Big (large digits)", "Большой (крупные цифры)"), t("Battery (readable at 16 px)", "Батарейка (читается на 16 px)"));
        setting_row_d(ui, &t("Mode", "Режим"), &t("Big: large digits / Battery: stylised icon", "Большой: крупные цифры / Батарейка: стилизованная иконка"), |ui| {
            egui::ComboBox::from_id_source("tray_mode")
                .selected_text(if self.tray_icon_config.mode == TrayIconMode::Big { m_big.as_str() } else { m_bat.as_str() })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.tray_icon_config.mode, TrayIconMode::Big, &m_big);
                    ui.selectable_value(&mut self.tray_icon_config.mode, TrayIconMode::Battery, &m_bat);
                });
            false
        });
        setting_row_d(ui, &t("Icon size", "Размер иконки"), &t("Preview resolution", "Разрешение превью"),
            |ui| ui.add(egui::Slider::new(&mut self.tray_icon_config.size, 16..=512)).changed());
        setting_row_d(ui, &t("Font scale", "Масштаб шрифта"), "",
            |ui| ui.add(egui::Slider::new(&mut self.tray_icon_config.font_scale, 10..=200)).changed());
        setting_row_d(ui, &t("Outline width", "Ширина обводки"), "",
            |ui| ui.add(egui::Slider::new(&mut self.tray_icon_config.outline_width, 0..=10)).changed());
        setting_row_d(ui, &t("Border width", "Ширина рамки"), "",
            |ui| ui.add(egui::Slider::new(&mut self.tray_icon_config.border_width, 0..=10)).changed());
        setting_row_d(ui, &t("Digit gap", "Расстояние между цифрами"), "",
            |ui| ui.add(egui::Slider::new(&mut self.tray_icon_config.gap_between_digits, 0..=10)).changed());
        setting_row_d(ui, &t("Smooth colour blend", "Плавная смена цвета"),
            &t("Colour changes smoothly from Low through Medium to High with the charge instead of jumping at thresholds. Charging keeps its own colour.", "Цвет меняется плавно от «Низкий» через «Средний» к «Высокий» вместе с зарядом, а не скачком на порогах. Зарядка сохраняет свой цвет."),
            |ui| ui.checkbox(&mut self.tray_icon_config.smooth_colors, "").changed());

        section_title(ui, &t("Charge thresholds", "Пороги заряда"));
        setting_row_d(ui, &t("High charge — above", "Высокий заряд — выше"), &t("Above this value the icon is \"High\" (default 50)", "Выше этого значения иконка «Высокий» (по умолчанию 50)"),
            |ui| ui.add(egui::DragValue::new(&mut self.tray_icon_config.high_threshold).range(1..=100)).changed());
        setting_row_d(ui, &t("Low charge — up to", "Низкий заряд — не выше"), &t("Between thresholds is \"Medium\"; at this value and below \"Low\" (default 20)", "Между порогами «Средний», на этом значении и ниже «Низкий» (по умолчанию 20)"),
            |ui| ui.add(egui::DragValue::new(&mut self.tray_icon_config.low_threshold).range(0..=99)).changed());

        section_title(ui, &t("Colours by status: BACKGROUND / TEXT / OUTLINE / BORDER", "Цвета по статусам: ФОН / ТЕКСТ / ОБВОДКА / РАМКА"));
        let names = [t("Charging", "Зарядка"), t("High", "Высокий"), t("Medium", "Средний"), t("Low", "Низкий"), t("Headset off", "Гарнитура выключена")];
        color_row(ui, &names[0], &mut self.tray_icon_config.colors.charging);
        color_row(ui, &names[1], &mut self.tray_icon_config.colors.high);
        color_row(ui, &names[2], &mut self.tray_icon_config.colors.medium);
        color_row(ui, &names[3], &mut self.tray_icon_config.colors.low);
        color_row(ui, &names[4], &mut self.tray_icon_config.colors.offline);

        ui.add_space(8.0);
        if ui.button(self.i18n.pick("Save tray icon config", "Сохранить настройки иконки")).clicked() {
            log::info!("[GUI] Save Tray Icon Config clicked");
            self.tray_icon_config.sanitize();
            if let Err(e) = self.tray_icon_config.save(crate::tray::icon::TrayIconConfig::default_path()) {
                log::warn!("Failed to save tray icon config: {}", e);
            }
            if let Some(tray) = &mut self.tray {
                tray.update_icon_config(self.tray_icon_config.clone());
                log::info!("[GUI] Tray icon config updated and refreshed");
            }
        }
        if ui.button(self.i18n.pick("Reset to default", "Сбросить по умолчанию")).clicked() {
            log::info!("[GUI] Reset tray icon to default");
            self.tray_icon_config = crate::tray::icon::TrayIconConfig::default();
            self.needs_save = true;
        }
        if ui.button(self.i18n.pick("Apply to tray now", "Применить к трею")).clicked() {
            log::info!("[GUI] Apply to Tray Now clicked");
            self.tray_icon_config.sanitize();
            if let Some(tray) = &mut self.tray {
                // update_battery() is a no-op when percent/charging did not change,
                // so push the whole config to force the icon to be re-rendered.
                tray.update_icon_config(self.tray_icon_config.clone());
            }
            self.tray_cfg_pending_save = true;
        }

        // Live apply: any slider/colour/reset change re-renders the tray icon immediately
        // and is written to tray_icon.toml once the mouse is released.
        if self.tray_icon_config != tray_cfg_before {
            self.tray_icon_config.sanitize();
            if let Some(tray) = &mut self.tray {
                tray.update_icon_config(self.tray_icon_config.clone());
            }
            self.tray_cfg_pending_save = true;
        }
        if self.tray_cfg_pending_save && !ui.input(|i| i.pointer.any_down()) {
            match self.tray_icon_config.save(crate::tray::icon::TrayIconConfig::default_path()) {
                Ok(_) => log::info!("[GUI] Tray icon config saved"),
                Err(e) => log::warn!("Failed to save tray icon config: {}", e),
            }
            self.tray_cfg_pending_save = false;
        }
    }

    fn show_settings_discord(&mut self, ui: &mut egui::Ui) {
        discord_tab::show(ui, &self.i18n, &mut self.config.discord, self.discord_connected, &mut self.needs_save, &mut self.recording_keybind, &mut self.keybind_capture);
    }

    fn scan_audio_devices(&mut self) {
        self.last_audio_device_scan = Instant::now();
        match crate::system_audio::get_audio_devices() {
            Ok(devices) => {
                self.audio_inputs = devices.iter().filter(|d| d.is_input).cloned().collect();
                self.audio_devices = devices.into_iter().filter(|d| d.is_output).collect();
            }
            Err(e) => log::error!("[GUI] Audio scan failed: {}", e),
        }
    }

    /// Dropdown of the system audio devices; stores the device id in `value`.
    fn audio_picker(ui: &mut egui::Ui, id: &str, none_label: &str, list: &[crate::system_audio::AudioDevice], value: &mut Option<String>) -> bool {
        let before = value.clone();
        let current = match value.as_deref() {
            None => none_label.to_string(),
            Some(v) => list.iter().find(|d| d.id == v).map(|d| d.name.clone()).unwrap_or_else(|| v.to_string()),
        };
        egui::ComboBox::from_id_source(id)
            .width(360.0)
            .selected_text(current)
            .show_ui(ui, |ui| {
                ui.selectable_value(value, None, none_label);
                for d in list {
                    ui.selectable_value(value, Some(d.id.clone()), d.name.clone());
                }
            });
        *value != before
    }

    fn show_settings_audio(&mut self, ui: &mut egui::Ui) {
        if self.audio_devices.is_empty() && self.audio_inputs.is_empty() && self.last_audio_device_scan.elapsed() >= Duration::from_secs(2) {
            self.scan_audio_devices();
        }
        let t = |en: &str, ru: &str| self.i18n.pick(en, ru);
        section_title(ui, &t("Fallback audio devices", "Резервные аудиоустройства"));
        ui.label(egui::RichText::new(t("Switch to the fallback devices automatically when the headset is off or charging.", "Автоматически переключаться на резервные устройства когда гарнитура отключена или на зарядке.")).size(12.0).color(egui::Color32::from_rgb(0x88, 0x88, 0x88)));
        let none = t("None", "Нет");
        let auto = t("Auto (find by HyperX name)", "Авто (найти по имени HyperX)");
        let (outs, ins) = (self.audio_devices.clone(), self.audio_inputs.clone());
        let mut changed = false;
        let rows = [
            (t("Fallback speakers", "Резервные динамики"), t("Use when the headset is off", "Использовать когда гарнитура отключена"), 0u8),
            (t("Fallback microphone", "Резервный микрофон"), t("Use when the headset is off", "Использовать когда гарнитура отключена"), 1),
            (t("Headset speakers", "Динамики гарнитуры"), t("Where to return sound when the headset turns on", "Куда вернуть звук, когда гарнитура включилась"), 2),
            (t("Headset microphone", "Микрофон гарнитуры"), t("Where to return the microphone when the headset turns on", "Куда вернуть микрофон, когда гарнитура включилась"), 3),
        ];
        for (title, desc, kind) in rows {
            let (list, slot, lbl) = match kind {
                0 => (&outs, &mut self.config.audio.fallback_output_device, &none),
                1 => (&ins, &mut self.config.audio.fallback_input_device, &none),
                2 => (&outs, &mut self.config.audio.headset_output_device, &auto),
                _ => (&ins, &mut self.config.audio.headset_input_device, &auto),
            };
            changed |= setting_row_d(ui, &title, &desc, |ui| Self::audio_picker(ui, &format!("aud{}", kind), lbl, list, slot));
        }
        changed |= setting_row_d(ui, &t("Auto-switch", "Автоматическое переключение"), &t("Switch devices automatically", "Переключать устройства автоматически"),
            |ui| ui.checkbox(&mut self.config.audio.auto_switch_audio, "").changed());
        ui.add_space(8.0);
        if ui.button(t("Refresh device list", "Обновить список устройств")).clicked() {
            self.scan_audio_devices();
        }
        if changed { self.needs_save = true; }
    }

    fn show_settings_debug(&mut self, ui: &mut egui::Ui) {
        let t = |en: &str, ru: &str| self.i18n.pick(en, ru);
        let (h, ms_u, s_u) = (t("Debug and logging", "Отладка и логирование"), t("ms", "мс"), t("s", "с"));
        section_title(ui, &h);
        let mut ch = false;
        ch |= setting_row_d(ui, &t("Move app streams", "Переносить потоки приложений"),
            &t("After the audio device changes, move playing apps to the new device (if music stayed on the old one)", "После смены устройства звука перенести играющие приложения на новое устройство (если музыка осталась на старом)"),
            |ui| ui.checkbox(&mut self.config.audio.move_app_streams, "").changed());
        ch |= setting_row_d(ui, &t("Pause between announcements", "Пауза между оповещениями"),
            &t("How long to wait between two voice messages so they do not overlap (ms)", "Сколько ждать между двумя голосовыми сообщениями, чтобы они не накладывались (мс)"),
            |ui| { let r = ui.add(egui::DragValue::new(&mut self.config.voice.announce_gap_ms).range(0..=5000).suffix(format!(" {ms_u}"))); r.changed() });
        ch |= setting_row_d(ui, &t("Headset status polling", "Опрос статуса гарнитуры"),
            &t("How often to ask whether the headset is on (s). Lower = faster audio switching, but the headset may take longer to sleep", "Как часто спрашивать, включена ли гарнитура (сек). Меньше — быстрее переключение звука, но гарнитура может дольше не засыпать"),
            |ui| ui.add(egui::DragValue::new(&mut self.config.status_poll_secs).range(1..=60).suffix(format!(" {s_u}"))).changed());
        ch |= setting_row_d(ui, &t("Delay before audio switch", "Задержка перед переключением звука"),
            &t("How many seconds the headset must stay off before audio moves to the fallback device (0 = immediately)", "Сколько секунд гарнитура должна оставаться выключенной, прежде чем звук уйдёт на резервное устройство (0 — сразу)"),
            |ui| ui.add(egui::DragValue::new(&mut self.config.offline_debounce_secs).range(0..=60).suffix(format!(" {s_u}"))).changed());
        ch |= setting_row_d(ui, &t("Debug log", "Отладочный лог"), &t("Verbose Rust logs", "Подробные логи Rust"),
            |ui| ui.checkbox(&mut self.config.debug_logging, "").changed());
        ch |= setting_row_d(ui, &t("Log to console", "Лог в консоль"), "",
            |ui| ui.checkbox(&mut self.config.log_to_console, "").changed());
        ch |= setting_row_d(ui, &t("Log to file", "Лог в файл"), &t("Write hyperx-ngenuity-open.log", "Записывать hyperx-ngenuity-open.log"),
            |ui| ui.checkbox(&mut self.config.log_to_file, "").changed());
        ui.add_space(6.0);
        ui.small(t("Logging changes require restart.", "Изменения логов применяются после перезапуска."));
        if ch { self.needs_save = true; }
    }

    fn show_settings_interface(&mut self, ui: &mut egui::Ui) {
        let t = |en: &str, ru: &str| self.i18n.pick(en, ru);
        section_title(ui, &t("Interface and startup", "Интерфейс и запуск"));
        let mut ch = false;
        ch |= setting_row_d(ui, &t("System notifications", "Системные уведомления"),
            &t("Desktop notifications with the battery level on connect, app start and low battery", "Уведомления Windows с уровнем заряда: при подключении, запуске и низком заряде"),
            |ui| ui.checkbox(&mut self.config.desktop_notifications, "").changed());
        ch |= setting_row_d(ui, &t("Start with the system", "Запускать с системой"), &t("Start the app at login", "Запускать приложение при входе в систему"),
            |ui| ui.checkbox(&mut self.config.start_with_os, "").changed());
        ch |= setting_row_d(ui, &t("Start in compact mode", "Запускать в компактном режиме"), &t("Open the small window on start", "Открывать маленькое окно при запуске"),
            |ui| ui.checkbox(&mut self.config.start_in_compact_mode, "").changed());
        ch |= setting_row_d(ui, &t("Compact mode", "Компактный режим"), &t("Show the compact window instead of the full one", "Показывать компактное окно вместо полного"),
            |ui| ui.checkbox(&mut self.config.compact_mode, "").changed());
        let mut selected = self.config.language.clone();
        let lang_title = t("Language", "Язык");
        setting_row_d(ui, &lang_title, &t("Interface language", "Язык интерфейса"), |ui| {
            egui::ComboBox::from_id_source("lang")
                .selected_text(if selected == "ru" { "Русский" } else { "English" })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut selected, "ru".to_string(), "Русский");
                    ui.selectable_value(&mut selected, "en".to_string(), "English");
                });
            false
        });
        if selected != self.config.language {
            let lang_dir = std::path::PathBuf::from("lang");
            self.config.language = selected.clone();
            self.i18n = crate::i18n::I18n::new(&lang_dir, &selected);
            ch = true;
        }
        if ch { self.needs_save = true; }
    }

}

/// Tab with a red underline when selected (matches the Tauri UI).
fn color_row(ui: &mut egui::Ui, title: &str, c: &mut crate::tray::icon::IconColors) {
    ui.separator();
    ui.horizontal(|ui| {
        ui.set_min_height(36.0);
        ui.label(egui::RichText::new(title).size(14.0).color(egui::Color32::from_rgb(0xd5, 0xd5, 0xd5)));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            for slot in [&mut c.border, &mut c.outline, &mut c.fg, &mut c.bg] {
                let mut col = egui::Color32::from_rgba_unmultiplied(slot[0], slot[1], slot[2], slot[3]);
                if egui::color_picker::color_edit_button_srgba(ui, &mut col, egui::color_picker::Alpha::BlendOrAdditive).changed() {
                    let [r, g, b, a] = col.to_srgba_unmultiplied();
                    *slot = [r, g, b, a];
                }
            }
        });
    });
}

/// Plain text from the system clipboard (used to paste the Discord connection code).
pub(crate) fn clipboard_text() -> Option<String> {
    #[cfg(windows)]
    {
        let out = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", "Get-Clipboard -Raw"])
            .creation_flags_hidden()
            .output()
            .ok()?;
        return Some(String::from_utf8_lossy(&out.stdout).to_string());
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[cfg(windows)]
trait HiddenCmd {
    fn creation_flags_hidden(&mut self) -> &mut Self;
}
#[cfg(windows)]
impl HiddenCmd for std::process::Command {
    fn creation_flags_hidden(&mut self) -> &mut Self {
        use std::os::windows::process::CommandExt;
        self.creation_flags(0x0800_0000)
    }
}

pub(crate) fn section_title(ui: &mut egui::Ui, text: &str) {
    ui.add_space(8.0);
    ui.label(egui::RichText::new(text.to_uppercase()).size(13.0).color(egui::Color32::from_rgb(0xaa, 0xaa, 0xaa)));
    ui.add_space(4.0);
}

/// Settings row with a grey description under the title (like `.setting-row` + `.setting-desc`).
pub(crate) fn setting_row_d(ui: &mut egui::Ui, title: &str, desc: &str, add: impl FnOnce(&mut egui::Ui) -> bool) -> bool {
    ui.separator();
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.set_min_height(46.0);
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(title).size(14.0).color(egui::Color32::from_rgb(0xd5, 0xd5, 0xd5)));
            if !desc.is_empty() {
                ui.label(egui::RichText::new(desc).size(12.0).color(egui::Color32::from_rgb(0x88, 0x88, 0x88)));
            }
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            changed = add(ui);
        });
    });
    changed
}

/// One settings row: title on the left, control on the right, separator above (like `.setting-row`).
fn setting_row(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui) -> bool) -> bool {
    ui.separator();
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.set_min_height(40.0);
        ui.label(egui::RichText::new(title).size(14.0).color(egui::Color32::from_rgb(0xd5, 0xd5, 0xd5)));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            changed = add(ui);
        });
    });
    changed
}

fn underline_tab(ui: &mut egui::Ui, selected: bool, text: &str) -> egui::Response {
    let color = if selected { egui::Color32::WHITE } else { egui::Color32::from_rgb(0xa8, 0xa8, 0xa8) };
    let galley = ui.painter().layout_no_wrap(text.to_string(), egui::FontId::proportional(14.0), color);
    let size = egui::vec2(galley.size().x + 16.0, 34.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let color = if resp.hovered() && !selected { egui::Color32::WHITE } else { color };
    ui.painter().galley(egui::pos2(rect.left() + 8.0, rect.center().y - galley.size().y / 2.0), galley, color);
    if selected {
        let y = rect.bottom();
        ui.painter().line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            egui::Stroke::new(2.0, egui::Color32::from_rgb(0xd4, 0x00, 0x00)),
        );
    }
    resp
}

/// Colors, spacing and type sizes taken from ui/styles.css so both front-ends look alike.
fn apply_tauri_theme(ctx: &egui::Context) {
    use egui::{Color32 as C, FontFamily::Proportional, FontId, TextStyle};
    let accent = C::from_rgb(0xd4, 0x00, 0x00);
    let mut v = egui::Visuals::dark();
    v.panel_fill = C::from_rgb(0x18, 0x18, 0x18);
    v.window_fill = C::from_rgb(0x1d, 0x1d, 0x1d);
    v.extreme_bg_color = C::from_rgb(0x22, 0x22, 0x22);
    v.faint_bg_color = C::from_rgb(0x1d, 0x1d, 0x1d);
    v.override_text_color = Some(C::from_rgb(0xd9, 0xd9, 0xd9));
    v.selection.bg_fill = accent;
    v.hyperlink_color = C::from_rgb(0xe0, 0x4a, 0x4a);
    let border = egui::Stroke::new(1.0, C::from_rgb(0x37, 0x37, 0x37));
    v.widgets.noninteractive.bg_stroke = border;
    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, C::from_rgb(0xaa, 0xaa, 0xaa));
    for (w, bg) in [
        (&mut v.widgets.inactive, C::from_rgb(0x4b, 0x4b, 0x4b)),
        (&mut v.widgets.hovered, C::from_rgb(0x5a, 0x5a, 0x5a)),
        (&mut v.widgets.active, accent),
    ] {
        w.bg_fill = bg;
        w.weak_bg_fill = bg;
        w.bg_stroke = egui::Stroke::new(1.0, C::from_rgb(0x55, 0x55, 0x55));
        w.rounding = egui::Rounding::same(2.0);
    }
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, C::from_rgb(0xdd, 0xdd, 0xdd));
    v.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, C::WHITE);
    ctx.set_visuals(v);
    ctx.style_mut(|s| {
        s.text_styles = [
            (TextStyle::Heading, FontId::new(18.0, Proportional)),
            (TextStyle::Body, FontId::new(14.0, Proportional)),
            (TextStyle::Button, FontId::new(14.0, Proportional)),
            (TextStyle::Small, FontId::new(12.0, Proportional)),
            (TextStyle::Monospace, FontId::new(13.0, egui::FontFamily::Monospace)),
        ]
        .into();
        s.spacing.item_spacing = egui::vec2(8.0, 8.0);
        s.spacing.button_padding = egui::vec2(10.0, 4.0);
    });
}
