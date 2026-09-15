# Asiba

Настольное приложение для мониторинга Linux-серверов. Без агентов: подключается по SSH, собирает всё, что умеет отдать сервер, и показывает в одном окне — список серверов, детали каждого, алерты, карту.

## Что умеет

- **Серверы** — карточки и таблица, поиск и фильтры по статусу, окружению, тегам. Добавление по хосту, пользователю и ключу (или без ключа — через ssh-agent и `~/.ssh`), jump host, sudo, ручное описание.
- **Модули** (показываются только те, у которых есть данные на сервере): система, CPU, память, диски, сеть, процессы, сервисы systemd, Docker/Podman, порты и файрвол, журнал, пользователи, обновления, проекты, безопасность, аномалии и DDoS, деплой, GPU.
- **Живые графики** с историей в SQLite (сырые точки 48 ч, поминутные 30 дней, почасовые год).
- **Действия** с подтверждением и журналом: бан IP (fail2ban / nftables / iptables / ufw), завершение процесса, рестарт и остановка юнита или контейнера.
- **Алерты** — встроенные правила плюс свои, базовая линия по ключевым метрикам, уведомления на рабочий стол.
- **Карта** — серверы по геолокации или ручным координатам, линии от вашей машины с задержкой пинга.
- **Локальная модель (аудит)** — Asiba сама запускает `llama-server` на `127.0.0.1` с указанным GGUF-файлом и по каждому инциденту (упавший юнит, контейнер, деплой, брутфорс, аномалия, диск, память…) или по кнопке «Полный аудит» просит модель объяснить причину и дать шаги. Контекст собирается из секций файла сервера по плейбуку инцидента плюс логи контейнера / journal юнита. Ничего не покидает машину. Отчёты — во вкладке «Аудит» сервера и в `~/.local/state/asiba/audits/<сервер>/`. Установка: `sudo dnf install llama-cpp`, модель — например [Qwen3-4B-Instruct-2507 Q4_K_M](https://huggingface.co/unsloth/Qwen3-4B-Instruct-2507-GGUF/blob/main/Qwen3-4B-Instruct-2507-Q4_K_M.gguf) (~2.5 ГБ, ~3 ГБ RAM), путь указывается в настройках. Подробности — `docs/LLM-PLAN.md`.
- **Инциденты** — единый список всего негативного по серверам на главной и в сводке сервера; каждый инцидент можно «разобрать» моделью.
- **Терминал** — кнопка `>_` на карточке и «Терминал» на странице сервера открывают системный эмулятор терминала с уже запущенным `ssh` (порт, jump host и ключ подставляются). Ищутся `$TERMINAL`, `x-terminal-emulator`, ptyxis, gnome-terminal, konsole, tilix, wezterm, xfce4-terminal, alacritty, kitty, foot, xterm.
- **Файлы сервера** — два файла в папке серверов, обновляются после каждого цикла сбора (не чаще раза в 10 с, только при изменениях). `<имя>.md` — для человека: всё, что показывает программа (проблемы, описание, система, ресурсы со сводкой за час и сутки, алерты, безопасность, аномалии, сервисы, Docker, деплои, проекты, процессы, порты, пользователи, обновления, журнал, GPU, действия, события); блок между `<!-- notes:start -->` и `<!-- notes:end -->` — ваш, не перезаписывается. `<имя>.llm.md` — те же данные в компактном виде на английском для локальной языковой модели (см. `docs/LLM-PLAN.md`).

## Сборка и запуск

Нужен Rust 1.98+ и системные библиотеки для окна (на Fedora: `libxkbcommon-devel wayland-devel`; для keyring — Secret Service, например GNOME Keyring или KWallet).

```
cargo run -p asiba-app --release
```

Проверки перед коммитом:

```
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --workspace
```

## Где что лежит

| Путь | Содержимое |
|---|---|
| `~/.config/asiba/config.toml` | тема, интервалы, сроки хранения, правила алертов, уведомления |
| `~/.config/asiba/servers/<имя>.toml` | описание сервера (без секретов) |
| `~/.config/asiba/servers/<имя>.md` | файл сервера для человека, обновляется автоматически |
| `~/.config/asiba/servers/<имя>.llm.md` | файл сервера для локальной модели |
| `~/.local/state/asiba/audits/<имя>/` | отчёты аудитов модели |
| системный keyring | пароли, passphrase, пароль sudo |
| `~/.local/share/asiba/history.db` | история метрик и журнал действий |
| `~/.local/state/asiba/layout.toml` | раскладка виджетов сводки |
| `~/.cache/asiba/` | геолокация, страны адресов, тайлы карты |

## Файл сервера `servers/<имя>.toml`

```toml
id = "neo"
host = "neo.example.org"
port = 22
user = "deploy"
sudo = "passwordless"          # none | passwordless | with_password

[auth]
kind = "key_file"              # auto | key_file | password
path = "/home/me/.ssh/neo"
has_passphrase = false

[jump]                         # необязательно
host = "bastion.example.org"
port = 22
user = "deploy"

[description]
project = "Shop"
purpose = "prod backend"
environment = "production"     # production | staging | development | other
tags = ["shop", "eu"]
owner = "ops@example.org"
links = ["https://grafana.example.org"]
notes = ""

[location]                     # необязательно; иначе — по IP
lat = 52.52
lon = 13.40
label = "Berlin, Hetzner"

[modules.deploy]               # настройки модулей, см. docs/MODULE_GUIDE.md
logs = "/var/log/deploy.log,/srv/shop/deploy.log"

[modules.processes]
interval = "5"                 # свой интервал сбора в секундах
enabled = "true"               # "false" выключает модуль на этом сервере
```

Секреты в файл не попадают: пароли вводятся в форме и уходят в keyring.

## Безопасность

- Ключ хоста проверяется по `~/.ssh/known_hosts`; незнакомый или изменившийся ключ показывается с отпечатком и требует явного подтверждения.
- Все команды модулей — только чтение; полный список в `docs/MODULE_GUIDE.md`. Действия (бан, kill, restart, stop) выполняются только через диалог подтверждения; опасные — с вводом имени сервера. Каждое действие записывается в журнал.
- Приложение не хранит команды с секретами и не логирует их.

## Для разработки

- `cargo run -p asiba-modules --example dump <модуль>` — снимок модуля с локальной машины.
- `cargo test -p asiba-modules --test live_local -- --ignored` — прогон всех модулей на localhost.
- `ASIBA_SCREENSHOT=/tmp/shot.png ASIBA_SCREENSHOT_DELAY=10 ASIBA_OPEN=<сервер>[/<вкладка>]|overview|servers|alerts|map|settings ASIBA_WINDOW=940x700 cargo run -p asiba-app` — снимок экрана нужной страницы при заданном размере окна и выход.

Документы: `docs/TZ-v1.md` (ТЗ), `docs/ARCHITECTURE.md`, `docs/MODULE_GUIDE.md` (как добавить модуль, реестр команд), `docs/DECISIONS.md` (журнал решений).
