use crate::tr_pick;
use notify_rust::Notification;

fn enabled() -> bool {
    crate::config::Config::load().map(|c| c.desktop_notifications).unwrap_or(true)
}

/// Low battery warning; `percent` is always included.
pub fn notify_low_battery(percent: u8) {
    if !enabled() {
        return;
    }
    let _ = Notification::new()
        .summary(&tr_pick("🔋 HyperX — низкий заряд", "🔋 HyperX — Low Battery"))
        .body(&tr_pick(
            &format!("Заряд гарнитуры {}%. Поставьте её на зарядку.", percent),
            &format!("Headset battery is {}%. Please charge it.", percent),
        ))
        .icon("battery-caution")
        .timeout(notify_rust::Timeout::Never)
        .show();
}

pub fn notify_full_charge() {
    if !enabled() {
        return;
    }
    let _ = Notification::new()
        .summary(&tr_pick("🔋 HyperX — заряжена", "🔋 HyperX — Fully Charged"))
        .body(&tr_pick("Гарнитура полностью заряжена.", "Your headset is fully charged."))
        .icon("battery-full")
        .timeout(notify_rust::Timeout::Never)
        .show();
}

/// Headset connected (or already connected when the app started): show the battery level.
pub fn notify_startup_battery(percent: u8, charging: bool) {
    if !enabled() {
        return;
    }
    let status = if charging {
        tr_pick("⚡ Заряжается", "⚡ Charging")
    } else {
        tr_pick("🔋 От батареи", "🔋 On battery")
    };
    let _ = Notification::new()
        .summary(&tr_pick("🎧 HyperX подключена", "🎧 HyperX Headset Connected"))
        .body(&tr_pick(
            &format!("{} — заряд {}%", status, percent),
            &format!("{} — {}% battery", status, percent),
        ))
        .icon("audio-headset")
        .timeout(notify_rust::Timeout::Milliseconds(5000))
        .show();
}
