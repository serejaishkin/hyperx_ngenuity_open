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
    ("Battery indicator (headphones + %):", "Индикатор заряда (наушники и %):"), ("Off", "Выключен"), ("Floating", "Плавающий"),
    ("On taskbar", "На панели задач"), ("Digits only", "Только цифры"), ("Percent only", "Только процент"),
    ("Headphones + %", "Наушники и %"), ("Mode:", "Режим:"),
    ("Big digits (fill the icon, no background)", "Крупные цифры (на всю иконку, без фона)"),
    ("Battery gauge (readable at 16px)", "Шкала батареи (читается в 16 px)"),
    ("Big mode: digits auto-fit the icon and use the status color; Outline is the outline color.", "Крупный режим: цифры подгоняются под иконку и берут цвет статуса; Outline — цвет контура."),
    ("Smooth color blend by charge", "Плавное смешение цвета по заряду"), ("Size:", "Размер:"), ("Outline:", "Контур:"),
    ("Colors", "Цвета"), ("Preview", "Превью"), ("High (>50%)", "Высокий (>50%)"), ("Medium (20-50%)", "Средний (20–50%)"),
    ("Low (<20%)", "Низкий (<20%)"), ("BG", "Фон"), ("FG", "Текст"), ("Out", "Конт."), ("Bdr", "Рамка"),
    ("Apply to Tray Now", "Применить к трею"), ("Save Tray Icon Config", "Сохранить настройки иконки"),
    ("Reset to Default", "Сбросить по умолчанию"), ("Voice Notifications", "Голосовые оповещения"),
    ("Apply Voice Settings", "Применить настройки голоса"),
    ("Fallback audio devices", "Резервные аудиоустройства"), ("Headset audio devices", "Устройства гарнитуры"),
    ("— not selected —", "— не выбрано —"), ("Fallback speakers", "Резервные динамики"),
    ("Fallback microphone", "Резервный микрофон"), ("Headset speakers", "Динамики гарнитуры"),
    ("Headset microphone", "Микрофон гарнитуры"), ("Auto-switch to fallback devices", "Автоматическое переключение"),
    ("Refresh device list", "Обновить список устройств"),
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
