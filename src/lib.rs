pub mod device;
pub mod devices;
pub mod input;
pub mod tray;
pub mod config;
pub mod audio;
pub mod system_audio;
pub mod discord;
pub mod platform;
pub mod i18n;
pub mod gui;
pub mod notifications;
pub mod dialog;
pub mod hotkey;
pub mod airoha_hid;
pub mod audio_switch;
pub mod audio_route;
pub mod low_battery;
pub mod logging;
pub mod single_instance;
#[cfg(target_os = "linux")]
pub mod airoha_race;
#[cfg(target_os = "linux")]
pub mod bluetooth;

pub use device::{DeviceCommand, DeviceEvent};

#[macro_export]
macro_rules! debug_println {
    ($($args:tt)*) => {
        if *$crate::VERBOSE.get().unwrap_or(&false) {
            println!($($args)*);
        }
    };
}

use std::sync::OnceLock;
pub static VERBOSE: OnceLock<bool> = OnceLock::new();

/// Pick the UI-language variant of a message (Russian when the config language is "ru"/unreadable).
pub fn tr_pick(ru: &str, en: &str) -> String {
    match config::Config::load() {
        Ok(c) if c.language != "ru" => en.to_string(),
        _ => ru.to_string(),
    }
}

/// Windows: the app used to be registered for autostart under the name "HyperHeadsetv2". After the
/// rename that entry would start a stale copy. Remove it; returns true when one was found so the caller
/// can register the current name instead.
#[cfg(windows)]
pub fn remove_legacy_autostart() -> bool {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
    use winreg::RegKey;
    match RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags("Software\\Microsoft\\Windows\\CurrentVersion\\Run", KEY_SET_VALUE) {
        Ok(run) => run.delete_value("HyperHeadsetv2").is_ok(),
        Err(_) => false,
    }
}

#[cfg(not(windows))]
pub fn remove_legacy_autostart() -> bool {
    false
}
