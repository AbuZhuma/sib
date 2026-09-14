pub const APP_NAME: &str = "ASIBA";

pub const NAV_OVERVIEW: &str = "Главная";
pub const NAV_SERVERS: &str = "Серверы";
pub const NAV_ALERTS: &str = "Алерты";
pub const NAV_MAP: &str = "Карта";
pub const NAV_SETTINGS: &str = "Настройки";

pub const STATUS_ONLINE: &str = "online";
pub const STATUS_OFFLINE: &str = "offline";
pub const STATUS_CONNECTING: &str = "подключение";
pub const STATUS_UNTRUSTED: &str = "ключ хоста";
pub const STATUS_LIVE: &str = "обновление";

pub const OVERVIEW_TITLE: &str = "Обзор";
pub const TILE_SERVERS: &str = "Серверов";
pub const TILE_ONLINE: &str = "Online";
pub const TILE_OFFLINE: &str = "Offline";
pub const TILE_EVENTS: &str = "Событий";
pub const SECTION_EVENTS: &str = "Последние события";
pub const SECTION_SERVERS: &str = "Серверы";
pub const EMPTY_EVENTS: &str = "Событий пока нет";
pub const EMPTY_SERVERS: &str = "Серверов пока нет. Добавьте первый.";

pub const SERVERS_TITLE: &str = "Серверы";
pub const BTN_ADD_SERVER: &str = "+ Добавить сервер";
pub const BTN_EDIT: &str = "Редактировать";
pub const BTN_RECONNECT: &str = "Переподключить";
pub const BTN_DELETE: &str = "Удалить";
pub const BTN_BACK: &str = "← Назад";
pub const BTN_SAVE: &str = "Сохранить";
pub const BTN_CANCEL: &str = "Отмена";
pub const BTN_TEST: &str = "Проверить подключение";
pub const BTN_TRUST_KEY: &str = "Доверять ключу и повторить";
pub const BTN_TRUST_KEY_SERVER: &str = "Доверять ключу";
pub const BTN_CONFIRM_DELETE: &str = "Удалить навсегда";

pub const FORM_TITLE_ADD: &str = "Новый сервер";
pub const FORM_TITLE_EDIT: &str = "Редактирование сервера";
pub const FORM_SECTION_CONNECTION: &str = "Подключение";
pub const FORM_SECTION_AUTH: &str = "Вход";
pub const FORM_SECTION_JUMP: &str = "Jump host";
pub const FORM_SECTION_SUDO: &str = "sudo";
pub const FORM_SECTION_DESCRIPTION: &str = "Описание";
pub const FORM_SECTION_TEST: &str = "Проверка";
pub const FORM_NAME: &str = "Имя";
pub const FORM_NAME_HINT: &str = "строчные латинские буквы, цифры, дефис";
pub const FORM_HOST: &str = "Хост";
pub const FORM_PORT: &str = "Порт";
pub const FORM_USER: &str = "Пользователь";
pub const FORM_AUTH_AUTO: &str = "Авто (ssh-agent, ~/.ssh/id_*)";
pub const FORM_AUTH_KEY: &str = "Приватный ключ";
pub const FORM_AUTH_PASSWORD: &str = "Пароль";
pub const FORM_KEY_PATH: &str = "Путь к ключу";
pub const FORM_PASSPHRASE: &str = "Passphrase";
pub const FORM_PASSWORD: &str = "Пароль";
pub const FORM_JUMP_ENABLE: &str = "Подключаться через промежуточный сервер";
pub const FORM_SUDO_NONE: &str = "Без sudo";
pub const FORM_SUDO_PASSWORDLESS: &str = "sudo без пароля";
pub const FORM_SUDO_PASSWORD: &str = "sudo с паролем";
pub const FORM_SUDO_PASSWORD_FIELD: &str = "Пароль sudo";
pub const FORM_PROJECT: &str = "Проект";
pub const FORM_PURPOSE: &str = "Назначение";
pub const FORM_ENVIRONMENT: &str = "Окружение";
pub const FORM_TAGS: &str = "Теги (через запятую)";
pub const FORM_OWNER: &str = "Ответственный";
pub const FORM_LINKS: &str = "Ссылки (по одной в строке)";
pub const FORM_NOTES: &str = "Заметки";
pub const FORM_SECRETS_KEPT: &str = "пусто — оставить сохранённый";

pub const ENV_PRODUCTION: &str = "production";
pub const ENV_STAGING: &str = "staging";
pub const ENV_DEVELOPMENT: &str = "development";
pub const ENV_OTHER: &str = "другое";

pub const TEST_IDLE: &str = "Подключение ещё не проверялось";
pub const TEST_RUNNING: &str = "Подключаемся…";
pub const TEST_OK: &str = "Подключение успешно";
pub const TEST_FAILED: &str = "Ошибка";
pub const TEST_MODULES: &str = "Модули";
pub const TEST_UNKNOWN_KEY: &str = "Сервер с таким ключом не встречался. Отпечаток:";
pub const TEST_CHANGED_KEY: &str = "Ключ хоста ИЗМЕНИЛСЯ. Возможна подмена сервера. Отпечаток:";

pub const AVAIL_AVAILABLE: &str = "доступен";
pub const AVAIL_PARTIAL: &str = "частично";
pub const AVAIL_UNAVAILABLE: &str = "недоступен";

pub const DETAIL_SECTION_CONNECTION: &str = "Соединение";
pub const DETAIL_SECTION_MODULES: &str = "Модули";
pub const DETAIL_SECTION_DESCRIPTION: &str = "Описание";
pub const DETAIL_RETRY_AT: &str = "повтор через";
pub const DETAIL_ONLINE_SINCE: &str = "в сети с";
pub const DETAIL_NO_DATA: &str = "данных пока нет";
pub const DETAIL_DELETE_PROMPT: &str = "Для подтверждения введите имя сервера:";

pub const COL_MODULE: &str = "Модуль";
pub const COL_STATUS: &str = "Статус";
pub const COL_UPDATED: &str = "Обновлено";
pub const COL_ERROR: &str = "Ошибка";
pub const COL_NAME: &str = "Имя";
pub const COL_HOST: &str = "Хост";
pub const COL_OS: &str = "ОС";
pub const COL_UPTIME: &str = "Uptime";
pub const COL_LOAD: &str = "Load";
pub const COL_TIME: &str = "Время";
pub const COL_SERVER: &str = "Сервер";
pub const COL_MESSAGE: &str = "Сообщение";

pub const MODULE_SYSTEM: &str = "Система";
pub const SYS_HOSTNAME: &str = "hostname";
pub const SYS_OS: &str = "ОС";
pub const SYS_KERNEL: &str = "ядро";
pub const SYS_UPTIME: &str = "uptime";
pub const SYS_LOAD: &str = "load average";
pub const SYS_CPU: &str = "CPU";
pub const SYS_MEMORY: &str = "RAM";
pub const SYS_SWAP: &str = "swap";
pub const SYS_VIRT: &str = "виртуализация";
pub const SYS_TIMEZONE: &str = "часовой пояс";
pub const SYS_CLOCK: &str = "расхождение часов";

pub const ALERTS_TITLE: &str = "Алерты";
pub const ALERTS_PLACEHOLDER: &str = "Правила и уведомления появятся на этапе 4.";
pub const MAP_TITLE: &str = "Карта";
pub const MAP_PLACEHOLDER: &str = "Геолокация и карта появятся на этапе 5.";

pub const SETTINGS_TITLE: &str = "Настройки";
pub const SETTINGS_SECTION_PATHS: &str = "Пути";
pub const SETTINGS_SECTION_APPEARANCE: &str = "Вид";
pub const SETTINGS_CONFIG_DIR: &str = "конфигурация";
pub const SETTINGS_SERVERS_DIR: &str = "файлы серверов";
pub const SETTINGS_DATA_DIR: &str = "данные";
pub const SETTINGS_THEME: &str = "Тема";
pub const SETTINGS_THEME_DARK: &str = "тёмная";
pub const SETTINGS_THEME_LIGHT: &str = "светлая";

pub const ERR_NAME_TAKEN: &str = "сервер с таким именем уже есть";
pub const ERR_HOST_EMPTY: &str = "укажите хост";
pub const ERR_USER_EMPTY: &str = "укажите пользователя";
pub const ERR_PORT: &str = "порт должен быть числом от 1 до 65535";
pub const ERR_KEY_PATH: &str = "укажите путь к ключу";
pub const ERR_JUMP: &str = "у jump host должны быть хост и пользователь";
