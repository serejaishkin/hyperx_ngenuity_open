//! Floating battery indicator (headphones + percent) as a small always-on-top egui viewport.
use super::HyperXApp;
use eframe::egui;
use parking_lot::Mutex;
use std::time::{Duration, Instant};

struct PosTrack {
    taskbar: bool,
    last: Option<(i32, i32)>,
    changed_at: Instant,
    saved: Option<(i32, i32)>,
}
static TRACK: Mutex<Option<PosTrack>> = Mutex::new(None);

#[cfg(windows)]
static TB_LAST_DEFAULT: Mutex<Option<(i32, i32)>> = Mutex::new(None);

enum Action {
    Open,
    Mute,
    Compact,
    Reset,
    Hide,
    Quit,
}

fn size_for(style: &str) -> (f32, f32) {
    match style {
        "digits" => (48.0, 30.0),
        "percent" => (64.0, 30.0),
        _ => (96.0, 30.0),
    }
}

fn draw_headphones(p: &egui::Painter, c: egui::Pos2, color: egui::Color32) {
    let r = 8.0;
    let stroke = egui::Stroke::new(2.0, color);
    let pts: Vec<egui::Pos2> = (0..=16)
        .map(|i| {
            let a = std::f32::consts::PI * (i as f32 / 16.0);
            egui::pos2(c.x - r * a.cos(), c.y - 1.0 - r * a.sin())
        })
        .collect();
    p.add(egui::Shape::line(pts, stroke));
    for dx in [-r, r] {
        let rect = egui::Rect::from_center_size(egui::pos2(c.x + dx, c.y + 2.0), egui::vec2(4.0, 8.0));
        p.rect_filled(rect, 1.5, color);
    }
}

impl HyperXApp {
    pub fn show_dock(&mut self, ctx: &egui::Context) {
        if !self.config.taskbar_widget {
            return;
        }
        let style = self.config.dock_style.clone();
        let (w, h) = size_for(&style);
        let mut builder = egui::ViewportBuilder::default()
            .with_title("HyperX battery")
            .with_inner_size([w, h])
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_resizable(false)
            .with_taskbar(false);
        let taskbar_mode = cfg!(windows) && self.config.dock_mode == "taskbar";
        let saved = if taskbar_mode { (self.config.dock_tb_x, self.config.dock_tb_y) } else { (self.config.dock_x, self.config.dock_y) };
        match saved {
            (Some(x), Some(y)) => builder = builder.with_position([x as f32, y as f32]),
            _ => {
                if let Some(m) = ctx.input(|i| i.viewport().monitor_size) {
                    builder = builder.with_position([m.x - w - 24.0, m.y - h - 64.0]);
                }
            }
        }

        let pct = self.device_state.battery_percent;
        let connected = self.device_state.connected;
        let charging = self.device_state.charging;
        let mut action: Option<Action> = None;
        let mut new_pos: Option<(i32, i32)> = None;
        let mute_txt = self.i18n.t("Mute");

        ctx.show_viewport_immediate(egui::ViewportId::from_hash_of("hx_dock"), builder, |ctx, _| {
            let color = if !connected {
                egui::Color32::GRAY
            } else if charging {
                egui::Color32::from_rgb(90, 160, 255)
            } else if pct <= 20 {
                egui::Color32::from_rgb(235, 70, 70)
            } else if pct <= 50 {
                egui::Color32::from_rgb(240, 180, 50)
            } else {
                egui::Color32::from_rgb(90, 210, 110)
            };
            let text = if connected { format!("{}%", pct) } else { "–".to_string() };
            let text = if style == "digits" { text.trim_end_matches('%').to_string() } else { text };

            egui::CentralPanel::default()
                .frame(egui::Frame::none().fill(egui::Color32::from_rgba_unmultiplied(24, 24, 28, 235)).rounding(8.0))
                .show(ctx, |ui| {
                    let rect = ui.max_rect();
                    let resp = ui.interact(rect, ui.id().with("hx_dock_i"), egui::Sense::click_and_drag());
                    if resp.drag_started() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                    }
                    if resp.double_clicked() {
                        action = Some(Action::Open);
                    }
                    let painter = ui.painter();
                    let mut tx = rect.center().x;
                    if style == "icon" || style.is_empty() {
                        draw_headphones(painter, egui::pos2(rect.left() + 22.0, rect.center().y), color);
                        tx = rect.left() + 58.0;
                    }
                    painter.text(
                        egui::pos2(tx, rect.center().y),
                        egui::Align2::CENTER_CENTER,
                        text,
                        egui::FontId::proportional(16.0),
                        color,
                    );
                    resp.context_menu(|ui| {
                        if ui.button(&self.i18n.t("Open")).clicked() { action = Some(Action::Open); ui.close_menu(); }
                        if ui.button(&mute_txt).clicked() { action = Some(Action::Mute); ui.close_menu(); }
                        if ui.button(&self.i18n.t("Compact window")).clicked() { action = Some(Action::Compact); ui.close_menu(); }
                        if ui.button(&self.i18n.t("Reset position")).clicked() { action = Some(Action::Reset); ui.close_menu(); }
                        if ui.button(&self.i18n.t("Hide indicator")).clicked() { action = Some(Action::Hide); ui.close_menu(); }
                        ui.separator();
                        if ui.button(&self.i18n.t("Quit")).clicked() { action = Some(Action::Quit); ui.close_menu(); }
                    });
                });

            if let Some(r) = ctx.input(|i| i.viewport().outer_rect) {
                new_pos = Some((r.min.x.round() as i32, r.min.y.round() as i32));
            }
        });

        // Persist the position once it has settled (user stopped dragging).
        if let Some(p) = new_pos {
            let mut guard = TRACK.lock();
            if guard.as_ref().map(|t| t.taskbar) != Some(taskbar_mode) {
                *guard = Some(PosTrack {
                    taskbar: taskbar_mode,
                    last: None,
                    changed_at: Instant::now(),
                    saved: match saved {
                        (Some(x), Some(y)) => Some((x, y)),
                        _ => None,
                    },
                });
            }
            let t = guard.as_mut().unwrap();
            if t.last != Some(p) {
                t.last = Some(p);
                t.changed_at = Instant::now();
            } else if t.saved != Some(p) && t.changed_at.elapsed() > Duration::from_millis(600) {
                t.saved = Some(p);
                if taskbar_mode {
                    self.config.dock_tb_x = Some(p.0);
                    self.config.dock_tb_y = Some(p.1);
                } else {
                    self.config.dock_x = Some(p.0);
                    self.config.dock_y = Some(p.1);
                }
                let _ = self.config.save();
            }
        }

        #[cfg(windows)]
        self.dock_taskbar_tick(ctx, taskbar_mode, w, h, saved.0.is_some() && saved.1.is_some());

        match action {
            Some(Action::Open) => {
                self.config.compact_mode = false;
                self.show_root(ctx);
            }
            Some(Action::Compact) => {
                self.config.compact_mode = true;
                self.show_root(ctx);
            }
            Some(Action::Mute) => {
                if let Some(tx) = &self.device_cmd_tx {
                    let _ = tx.send(crate::DeviceCommand::ToggleMute);
                }
            }
            Some(Action::Reset) => {
                if taskbar_mode {
                    self.config.dock_tb_x = None;
                    self.config.dock_tb_y = None;
                    #[cfg(windows)]
                    TB_LAST_DEFAULT.lock().take();
                } else {
                    self.config.dock_x = None;
                    self.config.dock_y = None;
                }
                if taskbar_mode { /* watcher recomputes the taskbar spot */ } else if let Some(m) = ctx.input(|i| i.viewport().monitor_size) {
                    let (w, h) = size_for(&self.config.dock_style);
                    ctx.send_viewport_cmd_to(
                        egui::ViewportId::from_hash_of("hx_dock"),
                        egui::ViewportCommand::OuterPosition(egui::pos2(m.x - w - 24.0, m.y - h - 64.0)),
                    );
                }
                if let Some(t) = TRACK.lock().as_mut() { t.saved = None; }
                let _ = self.config.save();
            }
            Some(Action::Hide) => {
                self.config.taskbar_widget = false;
                let _ = self.config.save();
            }
            Some(Action::Quit) => std::process::exit(0),
            None => {}
        }
    }

    /// Windows: overlay style + owner = taskbar (taskbar mode) or detach (floating); hide under full-screen apps.
    #[cfg(windows)]
    fn dock_taskbar_tick(&mut self, ctx: &egui::Context, taskbar_mode: bool, w: f32, h: f32, has_saved: bool) {
        use super::taskbar_dock as tb;
        static LAST_TICK: Mutex<Option<Instant>> = Mutex::new(None);
        static LAST_MODE: Mutex<Option<bool>> = Mutex::new(None);
        {
            let mut t = LAST_TICK.lock();
            if t.map(|i| i.elapsed() < Duration::from_millis(700)).unwrap_or(false) {
                return;
            }
            *t = Some(Instant::now());
        }
        let hv = tb::find_by_title("HyperX battery");
        if hv == 0 {
            return;
        }
        let dock_id = egui::ViewportId::from_hash_of("hx_dock");
        ctx.send_viewport_cmd_to(dock_id, egui::ViewportCommand::Visible(!tb::fullscreen_app_running()));
        let mode_changed = LAST_MODE.lock().replace(taskbar_mode) != Some(taskbar_mode);
        if mode_changed {
            TB_LAST_DEFAULT.lock().take();
        }
        let ppp = ctx.pixels_per_point().max(0.5);
        if taskbar_mode {
            tb::apply_overlay_style(hv);
            let Some(g) = tb::detect() else { return; };
            tb::ensure_above_taskbar(hv, g.taskbar_hwnd);
            if !has_saved {
                let (wp, hp) = ((w * ppp) as i32, (h * ppp) as i32);
                let pos = tb::default_position(&g, wp, hp, ppp as f64);
                let mut last = TB_LAST_DEFAULT.lock();
                if *last != Some(pos) {
                    *last = Some(pos);
                    ctx.send_viewport_cmd_to(dock_id, egui::ViewportCommand::OuterPosition(egui::pos2(pos.0 as f32 / ppp, pos.1 as f32 / ppp)));
                }
            } else if mode_changed {
                if let (Some(x), Some(y)) = (self.config.dock_tb_x, self.config.dock_tb_y) {
                    ctx.send_viewport_cmd_to(dock_id, egui::ViewportCommand::OuterPosition(egui::pos2(x as f32, y as f32)));
                }
            }
        } else {
            tb::detach_from_taskbar(hv);
            if mode_changed {
                if let (Some(x), Some(y)) = (self.config.dock_x, self.config.dock_y) {
                    ctx.send_viewport_cmd_to(dock_id, egui::ViewportCommand::OuterPosition(egui::pos2(x as f32, y as f32)));
                } else if let Some(m) = ctx.input(|i| i.viewport().monitor_size) {
                    ctx.send_viewport_cmd_to(dock_id, egui::ViewportCommand::OuterPosition(egui::pos2(m.x - w - 24.0, m.y - h - 64.0)));
                }
            }
        }
    }

    fn show_root(&mut self, ctx: &egui::Context) {
        let root = egui::ViewportId::ROOT;
        ctx.send_viewport_cmd_to(root, egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd_to(root, egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd_to(root, egui::ViewportCommand::Focus);
        self.window_hidden = false;
        let _ = self.config.save();
    }
}
