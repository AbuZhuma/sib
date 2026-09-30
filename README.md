# Sib

Мониторинг Linux-серверов в одном окне. Агент на сервер не ставится: приложение подключается по SSH, читает `/proc`, `/sys` и вывод обычных утилит, пишет историю в SQLite и в markdown-файл рядом с настройками сервера.

## Что умеет

- **Серверы.** Карточки и таблица, поиск и фильтры по статусу, окружению и тегам. Сервер добавляется по хосту, пользователю и ключу; без ключа берутся ssh-agent и `~/.ssh`. Поддерживаются jump host и sudo.
- **19 модулей.** Система, CPU, память, диски, сеть, процессы, сервисы systemd, Docker и Podman, порты и файрвол, журнал, пользователи, обновления, безопасность, аномалии и DDoS, деплой, git, GPU, файловый менеджер, свои проверки. Вкладка появляется, только если модуль доступен на сервере и в нём есть данные.
- **Графики** с историей в SQLite: сырые точки 48 часов, поминутные 30 дней, почасовые год.
- **Действия** с подтверждением и журналом: бан IP через fail2ban, nftables, iptables или ufw, завершение процесса, рестарт и остановка юнита или контейнера. Опасные требуют ввести имя сервера.
- **Алерты.** Десять встроенных правил; любое правится, выключается или возвращается к исходному, свои правила добавляются там же. Плюс базовая линия по ключевым метрикам и уведомления на рабочий стол.
- **Аудит.** 77 проверок безопасности и надёжности, оценка защиты от A до F. Вес и включение каждой проверки настраиваются отдельно для каждого сервера, туда же добавляются свои: команда, скрипт из формы, файл с этой машины или файл на сервере.
- **Инциденты.** Один список всего, что не в порядке, на главной и в карточке сервера.
- **Карта.** Серверы по геолокации или ручным координатам, линии от вашей машины с задержкой пинга.
- **ИИ-анализ.** Выключен по умолчанию, включается тумблером в настройках. Провайдеры: Gemini, OpenAI, Claude и любой OpenAI-совместимый API по URL (Groq, Mistral, DeepSeek, свой сервер). После включения модель разбирает каждый инцидент, анализирует разделы сервера по кнопке или автоматически и делает полный аудит. Отчёты лежат в `~/.local/state/sib/audits/`. Пока тумблер выключен, наружу ничего не уходит.
- **Терминал.** Кнопка открывает системный эмулятор терминала с запущенным `ssh`, порт и jump host подставляются. Ищутся `$TERMINAL`, `x-terminal-emulator`, ptyxis, gnome-terminal, konsole, tilix, wezterm, xfce4-terminal, alacritty, kitty, foot, xterm.
- **Файл сервера.** `<имя>.md` повторяет всё, что показывает программа, и обновляется после цикла сбора. Блок между `<!-- notes:start -->` и `<!-- notes:end -->` ваш, программа его не перезаписывает. Рядом `<имя>.llm.md`: те же данные компактно, для языковой модели.

## Установка

Сборка для x86_64 Linux лежит на странице [Releases](https://github.com/AbuZhuma/sib/releases):

```
curl -L https://github.com/AbuZhuma/sib/releases/latest/download/sib-x86_64-linux.tar.gz | tar xz
./sib
```

Нужен рабочий стол с Secret Service для keyring (GNOME Keyring, KWallet) и обычные библиотеки окна: `libxkbcommon`, `libwayland-client` или `libX11`, `libdbus-1`, драйвер OpenGL. SQLite собран внутрь бинарника.

## Сборка из исходников

Нужен Rust 1.88 или новее и заголовочные файлы тех же библиотек. Fedora: `libxkbcommon-devel wayland-devel dbus-devel`. Debian и Ubuntu: `libxkbcommon-dev libwayland-dev libdbus-1-dev`.

```
cargo run -p sib-app --release
```

Проверки перед коммитом; те же шаги гоняет CI в `.github/workflows/ci.yml`, плюс `cargo deny check` по `deny.toml`:

```
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --workspace
```

## Где что лежит

| Путь | Содержимое |
|---|---|
| `~/.config/sib/config.toml` | тема, интервалы, сроки хранения, правила алертов, уведомления, настройки ИИ без ключа |
| `~/.config/sib/servers/<имя>.toml` | настройки сервера без секретов |
| `~/.config/sib/servers/<имя>.md` | файл сервера для человека, обновляется автоматически |
| `~/.config/sib/servers/<имя>.llm.md` | файл сервера для языковой модели |
| `~/.local/state/sib/audits/<имя>/` | отчёты ИИ |
| `~/.local/state/sib/ignored.toml` | инциденты, которые вы убрали кнопкой «забыть» |
| `~/.local/share/sib/history.db` | история метрик и журнал действий |
| `~/.cache/sib/` | геолокация, страны адресов, тайлы карты |
| системный keyring | пароли, passphrase, пароль sudo, ключ API провайдера ИИ |

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

[location]                     # необязательно, иначе определяется по IP
lat = 52.52
lon = 13.40
label = "Berlin, Hetzner"

[modules.deploy]               # настройки модулей
logs = "/var/log/deploy.log,/srv/shop/deploy.log"

[modules.processes]
interval = "5"                 # свой интервал сбора в секундах
enabled = "true"               # "false" выключает модуль на этом сервере

[[checks]]                     # своя проверка аудита, правится в UI
id = "custom:1"
name = "Сертификат не истекает"
kind = "script"                # script | local_file | remote_file
source = "openssl x509 -checkend 604800 -noout -in /etc/ssl/site.pem"
expect = "exit_zero"           # exit_zero | contains | missing
area = "reliability"
weight = "high"                # low | medium | high
advice = "Продлить сертификат"

[check_overrides."ssh.port"]   # изменённая встроенная проверка
enabled = false
```

Секреты в файл не попадают: пароли вводятся в форме и уходят в keyring.

## Безопасность

- Ключ хоста проверяется по `~/.ssh/known_hosts`. Незнакомый или изменившийся показывается с отпечатком и требует подтверждения.
- Команды модулей только читают. Записывают лишь действия: бан, kill, restart, stop и правка файлов. Каждое проходит через диалог подтверждения и попадает в журнал.
- Приложение не логирует команды с секретами.
- Без включённого ИИ наружу уходят только запросы к `ip-api.com` для карты и стран адресов. Выключаются в Настройки → Общие.

## Для разработки

- `cargo run -p sib-modules --example dump <модуль>` снимает данные модуля с локальной машины.
- `cargo test -p sib-modules --test live_local -- --ignored` прогоняет все модули на localhost.
- `SIB_SCREENSHOT=/tmp/shot.png SIB_SCREENSHOT_DELAY=10 SIB_OPEN=<сервер>[/<вкладка>]|overview|servers|alerts|map|settings SIB_WINDOW=940x700 cargo run -p sib-app` снимает экран нужной страницы и выходит.
