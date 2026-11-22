use std::collections::HashMap;
use std::path::Path;

pub const DEFAULT_KEYS: &[&str] = &[];

const RU: &[(&str, &str)] = &[
    ("Dashboard", "Панель"), ("Equalizer", "Эквалайзер"), ("Settings", "Настройки"),
    ("Headset", "Гарнитура"), ("Voice", "Голос"), ("Tray Icon", "Трей"), ("Discord", "Discord"),
    ("Audio", "Аудио"), ("Debug", "Отладка"), ("Interface", "Интерфейс"),
    ("Headset Status", "Состояние гарнитуры"), ("Battery", "Батарея"), ("Microphone", "Микрофон"),
    ("Signal:", "Сигнал:"), ("Charging", "Зарядка"), ("Quick Actions", "Быстрые действия"),
    ("Mute Mic", "Выключить микрофон"), ("Unmute Mic", "Включить микрофон"),
    ("Check Battery (Voice)", "Проверить батарею (голос)"), ("MIC ON", "МИК ВКЛ"), ("MUTE", "МЬЮТ"),
    ("VOL", "ГРОМ"), ("MIC", "МИК"), ("Sidetone", "Сайдтон"), ("Sidetone (hear yourself)", "Сайдтон (слышать себя)"),
    ("Voice prompts", "Голосовые подсказки"), ("Auto-shutdown:", "Авто-выключение:"), ("Apply", "Применить"),
    ("Mute Button", "Кнопка мьюта"), ("Headset Settings", "Гарнитура"),
    ("Headset mute button mode:", "Режим кнопки мьюта:"), ("Play/Pause", "Воспр./Пауза"),
    ("Test: emulate press", "Тест: эмулировать нажатие"), ("Presets:", "Пресеты:"),
    ("Enable voice", "Включить голос"), ("Battery low", "Батарея разряжена"), ("Full charge", "Полная зарядка"),
    ("Button check", "Проверка кнопки"), ("Exact percent", "Точный процент"), ("Connected", "Подключено"),
    ("Disconnected", "Отключено"), ("Say percent on low battery", "Называть процент при низком заряде"),
    ("Warn at levels", "Предупреждать на уровнях"), ("Repeat at or below", "Повторять на уровне и ниже"),
    ("Audio Settings", "Аудио"), ("Language", "Язык"), ("Debug level (verbose)", "Отладочный лог"),
    ("Log to console", "Лог в консоль"), ("Logging changes require restart.", "Изменения логов применяются после перезапуска."),
    ("(!) Unsaved changes", "(!) Есть несохранённые изменения"), ("Open", "Открыть"), ("Mute", "Мьют"),
    ("Compact window", "Компактное окно"), ("Quit", "Выход"), ("Reset position", "Сбросить позицию"),
    ("Hide indicator", "Скрыть индикатор"), ("Interface and startup", "Интерфейс и запуск"),
    ("System notifications", "Системные уведомления"), ("Start with the system", "Запускать с системой"),
    ("Start in compact mode", "Запускать в компактном режиме"), ("Compact mode", "Компактный режим"),
    ("Headset connected", "Гарнитура подключена"), ("Device ready", "Устройство готово"),
    ("Save", "Сохранить"), ("Active", "Активен"), ("Muted", "Выключен"), ("OFF - No connection", "OFF — нет подключения"),
];

pub struct I18n {
    strings: HashMap<String, String>,
    lang: String,
}

impl I18n {
    pub fn new<P: AsRef<Path>>(_path: P, lang: &str) -> Self {
        Self { strings: HashMap::new(), lang: lang.to_string() }
    }

    pub fn t(&self, key: &str) -> String {
        if let Some(v) = self.strings.get(key) {
            return v.clone();
        }
        if self.lang == "ru" {
            if let Some((_, v)) = RU.iter().find(|(k, _)| *k == key) {
                return v.to_string();
            }
        }
        key.to_string()
    }

    pub fn current_lang(&self) -> &str {
        &self.lang
    }

    pub fn list_available<P: AsRef<Path>>(_path: P) -> Vec<(String, String)> {
        vec![("en".to_string(), "English".to_string()), ("ru".to_string(), "Русский".to_string())]
    }

    pub fn generate_default<P: AsRef<Path>>(_path: P, _keys: &[&str]) -> anyhow::Result<()> {
        Ok(())
    }
}
