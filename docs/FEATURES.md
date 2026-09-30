# Asiba — справочник возможностей

Что умеет приложение и как каждая возможность устроена внутри. Документ описывает состояние на 2026-09-22 (после исправлений по `AUDIT-2026-09-21.md`). Архитектурные принципы — в `ARCHITECTURE.md`, команды модулей — в `MODULE_GUIDE.md`, история решений — в `DECISIONS.md`, ИИ — в `AI-PLAN.md`. Здесь — полная карта: возможность → как работает → где код.

## 1. Общая картина

Asiba — настольное приложение (Rust, egui) для мониторинга Linux-серверов без агентов. К каждому серверу открывается одна SSH-сессия, по ней 19 модулей читают `/proc`, `/sys`, стандартные утилиты и свои проверки, результат показывается в одном окне, пишется в SQLite и в markdown-файл сервера, проверяется детекторами инцидентов и правилами алертов, а при включённом ИИ отправляется языковой модели на разбор.

| Слой | Крейт | Строк | Что делает |
|---|---|---|---|
| Домен | `asiba-core` | 1 625 | Типы: `ServerSpec`, `Transport`, `Module`, `Snapshot`, `AppState`, `Incident`, `Alert`, `AuditReport` |
| Транспорт | `asiba-transport` | 733 | SSH через `russh`, локальный транспорт, sudo, known_hosts, `~/.ssh/config` |
| Конфигурация | `asiba-config` | 968 | XDG-пути, `config.toml`, `servers/<id>.toml`, keyring, архив инцидентов |
| Хранилище | `asiba-storage` | 625 | SQLite: метрики (3 уровня), журнал действий, поток записи |
| Модули | `asiba-modules` | 10 192 | 18 сборщиков данных, парсеры, действия, запросы |
| Инциденты | `asiba-incidents` | 3 397 | 12 детекторов, 77 паттернов аудита и свои проверки, оценка безопасности, `reconcile` |
| Документы | `asiba-docgen` | 2 760 | 18 секций → `servers/<id>.md` (человек) и `<id>.llm.md` (модель) |
| Алерты | `asiba-alerts` | 657 | 10 встроенных правил, правка и свои правила, базовая линия EWMA, уведомления |
| ИИ | `asiba-ai` | 1 208 | Бэкенды Gemini / OpenAI / Anthropic, контекст с бюджетом, плейбуки, промпт, ретраи |
| Движок | `asiba-engine` | 2 976 | Воркеры серверов, циклы сбора, пинг, геолокация, алерты, действия, ИИ-очередь |
| UI | `asiba-ui` | 13 349 | Тема, страницы, представления модулей, файловый менеджер, карта |
| Приложение | `asiba-app` | 82 | Точка входа, tokio runtime, сборка зависимостей |

Итого 38 572 строки в 340 файлах, 277 тестов (плюс 3 live-теста под `--ignored`), 0 комментариев в коде (по правилам `CLAUDE.md`). CI — `.github/workflows/ci.yml` (fmt, clippy, tests, cargo-deny).

## 2. Серверы

### 2.1 Добавление и хранение

**Форма** (`asiba-ui/src/pages/server_form/`): имя (строчные латинские буквы, цифры, дефис, до 32 символов — `ServerId::parse`), хост, порт, пользователь, способ входа, jump host, sudo, описание (проект, назначение, окружение prod/stage/dev/other, теги, ответственный, ссылки, заметки), ручные координаты, настройки модулей. Кнопка «Проверить подключение» (`Command::TestConnection`) подключается, прогоняет `detect` всех модулей и собирает пробный снимок одного модуля; результат — список модулей с доступностью прямо в форме (`server_form/test.rs`).

**Способы входа** (`asiba-transport/src/ssh/auth.rs`):
- `auto` — ssh-agent → `IdentityFile` из `~/.ssh/config` → `~/.ssh/id_ed25519`, `id_ecdsa`, `id_rsa` → пароль, если задан;
- `key_file` — путь к ключу и опциональная passphrase;
- `password`.

`~/.ssh/config` (`ssh_config.rs`, крейт `ssh2-config`): для алиаса берутся `HostName` и `IdentityFile`; `Port` и `User` из формы имеют приоритет; `ProxyJump` не читается — jump host задаётся в форме и всегда входит по `auto`.

**Хранение**: спецификация — `~/.config/asiba/servers/<id>.toml` (`ServerStore`); переименование сервера в форме переносит файлы `.md`/`.llm.md`, секреты, архив инцидентов и состояние в памяти под новое имя (`Command::UpdateServer { previous }`), удаление стирает toml, оба документа, секреты и историю в SQLite. Секреты (пароль, passphrase, пароль sudo) — системный keyring, сервис `asiba`, аккаунты `<id>/password`, `<id>/passphrase`, `<id>/sudo` (`KeyringSecretStore`). Без Secret Service приложение работает, но входит без секретов (предупреждение в лог). При редактировании незаполненные секреты берутся из текущих (`Credentials::fill_missing_from`).

### 2.2 Жизненный цикл соединения

`asiba-engine/src/worker/mod.rs`, одна tokio-задача на сервер:

```
connect (таймаут 15 с)
  ├─ UnknownHostKey / HostKeyChanged → статус UntrustedHostKey, воркер останавливается,
  │    ждёт Command::TrustHostKey (диалог с отпечатком SHA-256 в UI, ключ пишется в known_hosts)
  ├─ ошибка → Offline { reason, retry_at }, backoff 5 → 10 → 30 → 60 с, повтор
  └─ ok → Online, detect_all (параллельно, JoinSet) → задача на каждый доступный модуль
        ├─ каждые 10 мин: все задачи останавливаются, detect_all заново
        └─ TransportError::Disconnected в любой задаче или в detect → все задачи стоп → Offline → backoff
```

При перезапуске воркера (переподключение, смена интервалов, правка сервера с тем же хостом) снимки, серии и лента событий сохраняются (`ServerState::restarted`).

Хост `localhost` / `127.0.0.1` / `::1` идёт через `LocalTransport` (`sh -c`, без SSH). SSH-сессия: keepalive 15 с × 3, `nodelay`, семафор на 4 одновременных канала (у `sshd` по умолчанию `MaxSessions 10`), таймаут команды 60 с. `ChannelOpenFailure` — ошибка команды, не обрыв; обрыв только `Disconnected`.

**sudo** (`sudo.rs`): `passwordless` → `sudo -n sh -c '…'`; `with_password` → `sudo -S -p '' sh -c '…'`, пароль в stdin. `Transport::exec_root` при `SudoMode::None` возвращает `SudoUnavailable`; `common::root::exec_prefer_root` пробует root и откатывается на обычного пользователя, `exec_as_root` — только root (или если уже root по `id -un`).

### 2.3 Список серверов и карточка

`pages/servers/`: сетка карточек или таблица, поиск по имени/хосту/проекту/тегам, фильтры по статусу (все / online / offline / с проблемами) и окружению. Карточка: имя, хост, окружение, теги, статус (online / offline / connecting / untrusted key), CPU %, RAM %, диск %, RX/TX, пинг, uptime, счётчики алертов, sparkline CPU, значки доступных модулей (`short_label`: sys, cpu, mem, dsk, net, prc, svc, dkr, prt, log, usr, upd, sec, anm, dpl, git, gpu, fil), кнопка `>_` (терминал).

### 2.4 Страница сервера

`pages/server_detail/`: шапка (статус, uptime, ОС, адрес, расположение, пинг; кнопки: файл сервера через `xdg-open`, терминал, редактировать, переподключить, удалить с вводом имени), вкладки только по доступным модулям с данными (`ModuleView::has_content`): Сводка, Процессы, Ресурсы (cpu + memory + disk), Сеть, Порты, Docker, Сервисы, Логи, Пользователи, Безопасность (всегда), Аномалии, Деплой, Git, GPU, Файлы.

**Сводка**: соединение, активные инциденты (кнопки «разобрать», «отчёт», «забыть»), блок ИИ «Полный аудит», сетка виджетов модулей в две колонки с кнопкой «Открыть», режим «Раскладка» (скрыть / вверх / вниз, сохраняется в `~/.local/state/asiba/layout.toml` — `LayoutStore`), таблица модулей (доступность, последний сбор, ошибка), описание.

**Панель «Просмотр»** (`pages/inspector.rs`): результат `Module::query` (логи контейнера, journal юнита, активность пользователя, история git) показывается над содержимым той вкладки, откуда запрошен.

### 2.5 Терминал

`app/external.rs`: `Action::OpenTerminal` собирает `ssh -p <port> [-J user@jump:port] [-i key] user@host` и запускает первый найденный эмулятор: `$TERMINAL`, `x-terminal-emulator`, ptyxis, gnome-terminal, konsole, tilix, wezterm, xfce4-terminal, alacritty, kitty, foot, xterm.

## 3. Модули

Модуль = папка `asiba-modules/src/<id>/` (`mod.rs` с `impl Module`, `model.rs`, `parse.rs` с тестами на `fixtures/<id>/`). Сбор — одна команда-скрипт `common::sections::script(&[(имя, команда)])`, вывод режется по маркерам `###имя` (`Sections::parse`). Модуль получает `CollectContext { previous, host, settings }` — предыдущий снимок для дельт, адрес подключения для внешних проверок, секцию `[modules.<id>]` из toml сервера.

Расписания (`Intervals`, по умолчанию, настраиваются в Настройках → Сбор; ограничения 1–60 / 5–600 / 30–3600 с; на сервер — `interval = "N"`, выключение — `enabled = "false"`):

| Расписание | Интервал | Модули |
|---|---|---|
| Fast | 2 с | cpu, memory, disk, network, processes, anomalies, gpu |
| Normal | 20 с | services, docker, ports, logs, users, security, deploy |
| Slow | 300 с | system, updates, git |
| OnDemand | — | files |

Каждый снимок даёт метрики `Sample { key, value }` (→ кольцо 900 точек в `ServerState.series` + SQLite) и события `Event` (→ лента). После 3 подряд ошибок сбора модуль отключается до следующего `detect` (10 мин); ошибка видна в таблице модулей и становится инцидентом `ModuleError`.

### 3.1 Таблица модулей

| Модуль | Доступность | Что собирает | Метрики (ключи) | События | Запросы / действия |
|---|---|---|---|---|---|
| `checks` | есть `timeout`; вкладка скрыта, пока у сервера нет проверок | свои проверки: команда или скрипт из настроек, скрипт с этой машины, скрипт на сервере; итог по коду возврата или по тексту в выводе | `checks.failed`, `checks.<id>.failed` | проверка перестала проходить (уровень из настройки), проверка снова проходит (info) | — |
| `system` | всегда | hostname, ОС, ядро, архитектура, uptime, load, CPU-модель и ядра, RAM/swap, виртуализация, часовой пояс, расхождение часов | — | — | — |
| `cpu` | всегда | `/proc/stat` по ядрам, PSI, частота, температура (thermal_zone, hwmon); загрузка считается по дельте с прошлым снимком | `cpu.total/user/system/iowait/steal`, `cpu.core.N`, `cpu.temperature` | — | — |
| `memory` | всегда | `/proc/meminfo`, PSI, `pswpin/pswpout`, `oom_kill` (плюс число OOM, замеченных с момента подключения, и время последнего) | `memory.used_pct/used_bytes/available_bytes/cached_bytes/swap_used_pct/swap_in_ps/swap_out_ps/pressure_some10` | OOM kill (critical) | — |
| `disk` | есть `df` | `df -P -B1` без tmpfs/overlay/…, inode, `/proc/diskstats` (скорости, IOPS, util), PSI io | `disk.root_used_pct`, `disk.fs.<mount>.used_pct`, `disk.io.<dev>.read_bps/write_bps/util_pct`, `disk.read_bps/write_bps/pressure_some10` | — | — |
| `network` | всегда | `/proc/net/dev`, `ip -o addr/link`, operstate/speed, соединения по состояниям (`ss`), шлюз | `network.rx_bps/tx_bps/rx_pps/tx_pps`, `network.<if>.rx_bps/tx_bps`, `network.conn.established/time_wait/syn_recv` | — | — |
| `processes` | всегда | все `/proc/[pid]/stat`, cmdline, io, пользователи (`ps`), CLK_TCK, PAGESIZE; CPU % и IO по дельте | `processes.count/running/zombies` | — | **terminate** (SIGTERM), **kill** (SIGKILL, опасное); PID > 1 |
| `services` | `systemctl` | все `.service` (состояние, рестарты, PID, время, путь юнита, result), таймеры (json) | `services.failed/active` | юнит → failed (critical), рестарт (warning) | запрос **journal** (`journalctl -u -n 200`); **restart / start / stop** (stop опасное), root |
| `docker` | `docker` или `podman` + доступ к сокету без sudo | версия, `ps -a`, `stats`, образы, `inspect` (рестарты, health, политика, exit code, compose-метки), тома, сети | `docker.running/stopped`, `docker.<name>.cpu_pct/mem_bytes` | остановлен (exit≠0 critical), рестарт (warning), unhealthy (critical) | запрос **logs** (`--tail 300 -t`); **restart / start / stop** |
| `ports` | `ss`; без sudo — Partial (нет файрвола) | `ss -tulpnH`, соединения по портам, файрвол (ufw / firewalld / nft / iptables → разрешённые порты), доступность снаружи: TCP-connect из приложения на публичные порты (≤64, таймаут 1,5 с, повтор раз в 5 мин при неизменном наборе) | `ports.listening/public` | новый порт (info), порт стал недоступен/доступен снаружи | — |
| `logs` | `journalctl`; без доступа к системному журналу — Partial | `journalctl -p warning -o json`: первый раз за час (300 записей), дальше `--after-cursor`; окно 24 ч / 2 000 записей, группировка одинаковых сообщений (цифры → `#`) | `logs.warnings_per_min/errors_per_min` (за 5 мин) | записи с priority ≤ 2 (critical), всплеск ≥ 30 за цикл (warning) | **backfill**: прокрутка вниз подгружает ещё 300 более старых (`--until=@…`) |
| `users` | всегда | `who`, `last -F -n 30`, `getent passwd`, группы sudo/wheel/admin, число ключей в `authorized_keys` | `users.sessions` | новый вход (info), новый пользователь (warning), новый ключ (warning) | запрос **activity `<user>`**: `.bash_history`, `.zsh_history` (хвост 300), `sudo` из журнала за 30 дней, `journalctl _UID=` (200 строк) |
| `updates` | apt-get / dnf / yum / pacman / zypper / apk | число обновлений, из них security, нужна ли перезагрузка (`/var/run/reboot-required`, `needs-restarting -r`) | `updates.pending/reboot_required` | — | — |
| `security` | всегда; без root — Partial | два яруса: журнал, sudo, fail2ban и баны — каждый сбор; `sshd -T`, хеши, hardening — раз в 5 мин. Неудачные и успешные входы SSH за 24 ч (journal или `auth.log`/`secure`), sudo-вызовы, fail2ban и джейлы, `sshd -T` (fallback — grep конфига), sha256 `/etc/passwd`, `/etc/group`, `/etc/sudoers`, `sudoers.d/*`, баны приложения (nft set `asiba`, цепочка `ASIBA`, ufw DENY), hardening: 18 ключей sysctl, SELinux/AppArmor, NTP, UID 0, пустые пароли, NOPASSWD, права `authorized_keys`, world-writable в `/etc`, опасные порты (21, 23, 512–514, 2375/2376, 6379, 27017, 9200), auditd, автообновления | `security.failed_logins/attackers/brute_force/bans` | брутфорс ≥ 10 неудач за 10 мин с IP (warning), вход (info; с нового адреса или root — warning), изменение passwd/group/sudoers (critical) | **ban** (`argument` — срок `30m`/`12h`/`7d` или навсегда; адрес, с которого Asiba подключена (`SSH_CLIENT`), забанить нельзя) / **unban**: fail2ban (если есть джейл) → nftables (таблица `inet asiba`, set с timeout) → iptables (цепочка `ASIBA`) → ufw |
| `anomalies` | `ss` и `/proc/net/snmp` | `ss -Htan` (состояния, топ-10 IP по соединениям и SYN-RECV), `/proc/net/netstat` (SyncookiesSent, ListenDrops/Overflows), `/proc/net/snmp` (PassiveOpens, AttemptFails, UDP), pps из `/proc/net/dev`, conntrack | `anomalies.syn_recv/pps_in/pps_out/new_conn_per_s/top_share_pct/peers/attack` | новый признак атаки (уровень признака) | бан IP из ленты — действие модуля `security` |
| `deploy` | docker/podman, или юнит `deploy*`, или процесс CI-раннера; иначе скрыт | compose: `docker events` за окно с прошлого сбора (первый раз 24 ч) → стадии pull/create/start/healthy; systemd: `deploy*` юниты + `journalctl -u` (60 строк); лог-файлы из `[modules.deploy] logs = "a,b"` (`stat` + `tail -n 400`, стадии и ошибки регулярками: docker compose, npm, cargo, pip, git, systemctl); раннер GitHub Actions / GitLab (`pgrep`, cwd, `_diag/Worker_*.log`) | `deploy.active/failed` | начался (info), завершён (info), упал на стадии (critical) | — |
| `git` | `git` и хотя бы один `.git` в `/opt /srv /var/www /home/* /root /app /docker /data` (глубина 3) | по каждому репозиторию: ветка, HEAD, origin, ahead/behind, `status --porcelain` (≤200), stash, ветки (≤50), теги (≤20), 100 коммитов; `GIT_OPTIONAL_LOCKS=0`, `safe.directory=*` | `git.repositories/dirty` | новый коммит (info), переключение ветки (info) | запрос **history `<path>`** — `git log -2000` |
| `gpu` | `nvidia-smi` или AMD `gpu_busy_percent` | загрузка, память, температура, мощность, процессы на GPU | `gpu.util_pct`, `gpu.<i>.util_pct/mem_pct/temp_c/power_w` | ≥ 85 °C (warning), память ≥ 95 % (warning) | — |
| `files` | GNU `find -printf` и `stat` | ничего по расписанию | — | — | запросы **list `<dir>`** (≤1000 записей), **read `<file>`** (≤200 КБ, не бинарный), **search `<pattern>`** (`find / -iname`, `nice`, 15 с, ≤200); действия **write** (атомарно через временный файл; под системными каталогами — опасное), **mkdir, create, move, copy, chmod, chown, delete** (`rm -rf`, опасное); всё через sudo, если настроен |

### 3.2 Дельты и скорости

`common::rate::per_second(current, previous, elapsed)`: счётчики хранятся как есть, скорость появляется со второго снимка (первый показывает «—»). Сброс счётчика (current < previous) даёт 0. При повторном `detect` и переподключении цикл берёт предыдущий снимок из `AppState`, если он моложе трёх интервалов (`collect::recent_snapshot`), так что скорости не обнуляются.

### 3.3 Запросы, подгрузка, действия

- **Запрос** (`Module::query`): UI → `ViewAction::Query` → `Command::Query { token }` → движок берёт транспорт из слота воркера → результат `EngineEvent::QueryFinished` → панель «Просмотр» (или `FileBrowser` для `files`).
- **Подгрузка истории** (`Module::backfill`): `Command::Backfill` → `broadcast` в воркер → цикл сбора нужного модуля выполняет `backfill` вместо очередного `collect`, поэтому снимок не гоняется с живым сбором.
- **Действие** (`Module::perform`): `ViewAction::Act` → диалог подтверждения (`app/confirm.rs`: сервер, цель, для бана — срок; ввод имени сервера при `Danger::High` или окружении `production`) → `Command::Perform` → `engine/actions.rs` выполняет, пишет `ActionRecord` в SQLite (`actions`), в `AppState.actions` (200 последних) и шлёт уведомление в статусбар. Аргумент длиннее 120 символов в журнале заменяется на «N байт». Журнал — Настройки → Журнал действий.

## 4. Графики и история

- **В памяти**: `ServerState.series[key]` — кольцо на 900 точек (`asiba-core/src/series.rs`). UI рисует `TimeSeriesPlot` (`egui_plot`) и sparkline (`components/plot.rs`, `sparkline.rs`).
- **SQLite** (`~/.local/share/asiba/history.db`, WAL): поток `asiba-storage` в отдельном потоке ОС, батч раз в 5 с или 5 000 сэмплов. Таблицы `samples` (сырые, 48 ч) → `samples_1m` (avg/min/max, 30 дней) → `samples_1h` (365 дней); даунсэмплинг при старте, затем раз в час и при смене сроков (`maintenance.rs`). Сроки — `[retention]` в `config.toml`, тип `Retention` в `asiba-core`, применяются на лету (`Command::SetRetention`). Удаление сервера стирает его строки.
- **При старте воркера** (`engine/history.rs`): последние 30 минут сырых сэмплов из базы подставляются перед живыми точками (`Series::prepend_history`).
- **Пинг** (`worker/ping.rs`): TCP-connect на SSH-порт раз в 5 с (таймаут 3 с) с локальной машины → `ServerState.ping` и серия `ping.rtt_ms` (только в памяти). Для сервера за jump host пингуется сам jump host (в UI подпись «пинг (jump host)»).
- **Пауза**: кнопка в статусбаре замораживает копию `AppState` для UI; сбор продолжается.

## 5. События, алерты, инциденты

Три уровня «негатива», от сырого к агрегированному:

### 5.1 События

`Event { at, server, module, severity, message }` — порождаются модулями при изменении между снимками (см. таблицу модулей), движком (открытие инцидента). Хранятся в `AppState.events` (500) и `ServerState.recent_events` (500); `AppState.events` показываются на главной с фильтром по уровню, `recent_events` попадают в файл сервера. В базу не пишутся.

### 5.2 Алерты

`asiba-alerts`, цикл движка раз в 5 с (`engine/alerts.rs`):
- **Правила** `AlertRule { metric, condition Above/Below, threshold, for_secs, severity, enabled }` — по последнему значению серии; значение не учитывается (правило считается не сработавшим), если сервер не online, модуль метрики в ошибке или точка старше часа. Встроенные (`rules.rs`): offline (виртуальная метрика `connection.offline`, 60 с, critical), cpu-high (90 %, 5 мин), memory-high (90 %, 2 мин), disk-full (90 %, critical), disk-warning (80 %), services-failed, log-errors (30/мин), attack, brute-force, reboot-required (info). Настройки → Правила алертов (`pages/alert_rules/`) — одна таблица, где правятся все правила: название, метрика, условие, порог, длительность, уровень и флаг «вкл». Кнопка «Добавить правило» даёт пустую строку, «удалить» убирает своё правило, «сброс» возвращает встроенному значения по умолчанию. В `config.toml` `[[alert_rules]]` пишутся только правила, отличающиеся от встроенных; правило с id встроенного его заменяет, выключенное (`enabled = false`) не проверяется.
- **Базовая линия** (`baseline.rs`): EWMA среднего и дисперсии (полураспад 720 сэмплов, прогрев 120) по 8 метрикам (`cpu.total`, `network.rx_bps/tx_bps/conn.established`, `processes.count`, `logs.errors_per_min`, `security.failed_logins`, `anomalies.pps_in`); отклонение ≥ 4σ (σ не меньше 5 % от среднего и не меньше 1) дольше 60 с → warning «Аномалия <metric>».
- Активные и история (300) — страница «Алерты»: подтвердить, заглушить на N часов, перейти к серверу/вкладке. Уведомления на рабочий стол (`notify-rust`) для warning/critical, если включены. Алерты живут только в памяти.

### 5.3 Инциденты

`asiba-incidents` — единый список «что не так» для UI, docgen и ИИ. 12 детекторов (`detectors/`), каждый по `ServerState` + `AppState` возвращает `IncidentDraft { kind, severity, subject, summary, evidence }`:

| Вид | Источник | Уровень |
|---|---|---|
| Alert | активные алерты ≥ warning | уровень алерта |
| Anomaly | признаки атаки из `anomalies` | уровень признака |
| BruteForce | атакующие ≥ 10 неудач / 10 мин (+ страна, забанен ли) | critical |
| SecurityCheck | провалы паттернов аудита областей безопасности и своих проверок | Fail+High → critical, Fail / Warn+High → warning, Warn → info |
| UnitFailed | юниты в `failed` | critical |
| ContainerDown | остановлен с политикой перезапуска (critical); exit≠0 без политики (warning, пока статус не «days ago»); unhealthy или ≥ 5 рестартов (warning) | |
| DeployFailed | последний деплой проекта упал (+ 10 строк лога) | critical |
| DiskFull | ФС ≥ 90 % (critical), ≥ 80 % (warning), inode ≥ 90 % | |
| Memory | OOM kill, замеченный за последние 24 ч, swap ≥ 80 % | warning |
| Updates | security-обновления (warning), нужна перезагрузка (info) | |
| ModuleError | модуль с `last_error` | warning |
| Clock | расхождение часов > 60 с | warning |

`reconcile(&mut AppState, now)` каждые 5 с: черновики ≥ warning сравниваются с активными инцидентами по `kind + subject`; новые открываются (событие, уведомление, `EngineEvent::IncidentsOpened`, задание ИИ), исчезнувшие закрываются, совпавшие обновляются. **Архив**: кнопка «забыть» → `IgnoredIncident { server, kind, subject }` в `~/.local/state/asiba/ignored.toml`; такие черновики отбрасываются, а для вида `alert` пропускается и само правило. Список и «вернуть» — на странице «Алерты».

### 5.4 Аудит системы и оценка безопасности

`asiba-incidents/src/audit/`: 77 паттернов `Pattern { id, area, subject, description, weight, advice, evidence, evaluate }` в 11 областях: SSH (11), доступ и права (8), сеть и файрвол (8), ядро (17 sysctl), защита системы (4), ресурсы (9), надёжность (8), сетевые интерфейсы (2), обновления (4), журнал (3), сбор данных (3). `system_audit(server, state)` прогоняет все и даёт `AuditCheck` с исходом Pass / Warn / Fail / Skipped. Свои проверки из `config.toml` добавляются к паттернам в области «Свои проверки» (`audit/custom.rs`): исход берётся из результата модуля `checks`, вес — из уровня проверки (critical → High, warning → Medium). Область не входит в оценку безопасности, но провалы становятся инцидентами. `security_score` по областям безопасности: буква A–F (A ≥ 90 % без провалов High, B ≥ 75 %, C ≥ 60 %, D ≥ 40 %), Warn даёт половину веса.

Вкладка «Безопасность» (`pages/server_detail/security/`; результат `system_audit` кешируется на секунду, `components/audit_cache.rs`): подразделы «Аудит» (по умолчанию только проблемы, фильтр), «Входы и sudo», «Атаки и баны»; клик по проверке → страница проблемы: состояние, описание, совет, улика (содержимое файла через `files`, запрос модуля в «Просмотр» или переход на вкладку). Внизу — блок ИИ «Аудит».

## 6. Файл сервера

`asiba-docgen`: 18 секций (`sections/`), каждая умеет `human` (русский markdown) и `llm` (английский `key: value`, ограниченные списки): findings, description, system, resources (текущее + средние/макс за час и сутки; окно выводится только если серия реально его покрывает — иначе «—»), alerts, security, anomalies, services, docker, deploy, processes, ports, users, updates, logs, gpu, actions, events. `DocWriter` (`worker/docs.rs`) после каждого снимка любого модуля, не чаще раза в 10 с и только при изменении тела (без строки времени), пишет `servers/<id>.md` (блок между `<!-- notes:start -->` и `<!-- notes:end -->` сохраняется) и `servers/<id>.llm.md`. Пишется только когда есть снимок `system`.

## 7. Карта и геолокация

- Переключатель «Приватность» в настройках (`AppConfig.geolocation`, `Command::SetGeolocation`) выключает оба запроса ниже; ручные координаты работают всегда.
- `engine/geo.rs`: при старте воркера — ручные координаты из `[location]` → кеш `~/.cache/asiba/geo.json` (30 дней) → `http://ip-api.com/json/<ip>`; частные адреса считаются «своей» точкой (`self`). Положение локальной машины — тот же запрос без IP.
- `engine/peers.rs`: раз в 30 с страны для IP атакующих (≤30 на сервер) и топ-пиров аномалий пакетом до 100 через `ip-api.com/batch`; кеш `countries.json`; показываются в таблицах безопасности и аномалий.
- UI (`components/map.rs`, крейт `walkers`): OSM-тайлы с кешем в `~/.cache/asiba/tiles`, маркеры серверов цветом статуса, линии от локальной машины с подписью RTT; мини-карта на главной, полноэкранная — страница «Карта».

## 8. ИИ-анализ

Подробно — `AI-PLAN.md`. Кратко:

- **Включение**: Настройки → ИИ-анализ: тумблер вверху раздела (пока он выключен, остальные поля раздела неактивны и серые), рядом «?» с перечнем того, что уходит; провайдер Gemini / OpenAI / Claude / OpenAI-совместимый (свой `base_url`); ключ; модель из списка или своя; лимит контекста (60 000 токенов); автоанализ инцидентов с уровня; кулдаун (30 мин на `сервер:вид:объект`); автоанализ разделов и список разделов. Ключ хранится в keyring (аккаунт `ai/api_key`; ключ из старого `config.toml` переносится при первом запуске). Без `consent` наружу ничего не идёт.
- **Виды**: сводка по парку (при старте после первого сбора со всех серверов, при включении ИИ, по кнопке на главной), инцидент (автоматически при открытии — с логами контейнера / journal юнита), раздел (при первом открытии вкладки, если включено, и по кнопке), полный аудит (кнопка в сводке и на вкладке «Безопасность»). Плейбук задаёт секции, задачу и формат ответа (`asiba-ai/src/playbook/`).
- **Исполнение**: один воркер (`engine/ai/worker.rs`), очередь, отчёт сразу со статусом «в очереди», отмена (`Command::CancelAudit`), ретраи на 408/429/5xx через 3/10/30 с, запасные модели провайдера, пауза автозаданий на 15 мин при 429. Контекст: `ContextBuilder` режет части по приоритету (findings → запросы → секции). Отчёт — в `AppState.audits` и `~/.local/state/asiba/audits/<сервер>/<время>_<scope>.md`.
- **UI**: блок на главной («ИИ-сводка», «Разборы инцидентов»), блок в сводке сервера, «ИИ-анализ раздела» на каждой вкладке, вкладка/панель «Аудит», значок «ИИ анализирует» в статусбаре.

## 9. Файловый менеджер

Вкладка «Файлы» (`pages/server_detail/files/`, состояние в `FileBrowser` на сервер): дерево-аккордеон от `/`, иконки по типу, двойной клик раскрывает папку или открывает файл, правая кнопка — контекстное меню (открыть, обновить, новый файл/папка, переименовать, переместить, копировать, права, владелец, удалить), поиск по имени по всей ФС с раскрытием дерева до результата, редактор открытого файла с сохранением через подтверждение. Листинги кешируются до «обновить» или собственного действия; после успешного действия приложение само перезапрашивает родительскую папку и файл (`app/events.rs`).

## 10. Страницы и оболочка

- **Боковая панель**: Главная, Серверы, Алерты, Карта, Настройки; ниже — серверы с индикатором статуса.
- **Статусбар**: online/всего, подключаются, счётчики алертов по уровням, бейдж «атака», бейдж ИИ с размером очереди, до двух уведомлений (12 с), время последнего обновления, кнопка паузы.
- **Главная** (`pages/overview/`): плитки (серверы, online, с проблемами, суммарные CPU/RAM/диск/трафик, алерты), активные инциденты, ИИ-блоки, «Деплои сейчас», «Аномалии», таблица серверов, топ-5 по CPU/RAM/диску/трафику, мини-карта, лента событий (40, фильтр по уровню).
- **Настройки**: шесть разделов, переключаются чипами вверху страницы, на экране только выбранный: «Общие» (тема тёмная/светлая `theme/palette.rs`, геолокация), «Сбор и хранение» (интервалы, сроки), «ИИ-анализ», «Правила алертов» (правила и уведомления), «Журнал действий», «О программе» (версия и пути). Пояснения к настройкам не занимают место на странице: рядом с элементом стоит кнопка «?», подсказка раскрывается по наведению (`components/help.rs`, ширина окна подбирается под длину текста). У блока с собственными параметрами в заголовке стоит «⚙»: он открывает попап с этими параметрами (`components/block_settings.rs` — `gear`, `scrolled`, `popup`; содержимое прокручивается, строка действий закреплена внизу). Первый такой блок — «Свои проверки» на вкладке «Безопасность». Папка серверов задаётся только в `config.toml` (`servers_dir`).
- **Тема**: все цвета, радиусы (2 px), отступы, высоты строк — токены в `asiba-ui/src/theme/`; шрифты в `theme/fonts.rs`.

### 5.5 Свои проверки

Проверки задаются для каждого сервера отдельно: вкладка «Безопасность» карточки сервера, блок «Свои проверки» и кнопка «⚙» в его заголовке (`pages/server_detail/security/checks/`). В попапе — таблица проверок, форма правки (название, что запускать, условие «пройдена», уровень при провале, запуск от root, «что проверяется», «что делать при провале»), кнопки «Добавить проверку», «Импортировать» и «Применить». «Импортировать» показывает проверки остальных серверов; выбранная копируется в текущий сервер с новым id и сразу открывается на правку.

Хранятся в `servers/<имя>.toml` в `[[checks]]`. «Применить» шлёт `Command::SetServerChecks`, движок дописывает список в спецификацию сервера (`Persistence::save_spec`, секреты не трогаются) и перезапускает воркер этого сервера. Список приходит модулю в `CollectContext.checks`.

Что запускать:
- **команда или скрипт** — текст из формы передаётся серверу и выполняется через `sh`;
- **файл на этом компьютере** — файл читается на машине с Asiba при каждом запуске, поэтому правка скрипта применяется без перезапуска приложения; содержимое выполняется на сервере через `sh`;
- **файл на сервере** — выполняется `sh <путь>` на самом сервере.

Условие «пройдена»: код возврата 0, вывод содержит текст, вывод не содержит текст (в выводе объединены stdout и stderr, до 4000 символов). Флаг «запускать от root» шлёт команду через `exec_root`; без настроенного sudo такая проверка помечается «пропущена». Время работы одной проверки ограничено 20 секундами (`timeout`).

Результаты видны в том же блоке, на вкладке «Проверки» карточки сервера (`modules/checks.rs`) и в аудите; провал даёт событие, инцидент и метрику `checks.<id>.failed` (0/1), по которой можно завести правило алерта. Все проверки сервера выключаются строкой `[modules.checks] enabled = "false"` в `servers/<имя>.toml`.

## 11. Безопасность приложения

- Host key проверяется по `~/.ssh/known_hosts`; неизвестный или изменившийся показывается с отпечатком и требует явного доверия; после доверия ключ дописывается в `known_hosts`.
- Все команды модулей — чтение; записи только через `Module::perform` с подтверждением и журналом (содержимое записываемого файла в журнал не попадает — только размер). Запись файла — атомарная через временный файл; под `/etc`, `/boot`, `/usr`, `/bin`, `/sbin`, `/lib` требует ввода имени сервера. Цели действий валидируются (`validate_ip`, `validate_name`, PID > 1, абсолютный путь без `..`, восьмеричный режим, срок бана `\d+[smhd]`), всё экранируется `shell_quote`.
- Секреты — только в keyring; в toml не попадают. Команды с паролем sudo не логируются.
- Наружу без включения ИИ уходят только запросы к `ip-api.com` (IP серверов и атакующих; переключатель Настройки → Приватность, `geolocation` в `config.toml`) и OSM-тайлы карты.

## 12. Хранение на диске

| Путь | Содержимое | Кто пишет |
|---|---|---|
| `~/.config/asiba/config.toml` | тема, интервалы, сроки хранения, правила алертов, уведомления, ИИ | UI (Настройки) |
| `~/.config/asiba/servers/<id>.toml` | описание сервера | движок при сохранении формы |
| `~/.config/asiba/servers/<id>.md`, `<id>.llm.md` | файлы сервера | `DocWriter` |
| keyring `asiba` | пароли, passphrase, sudo, ключ API ИИ | `Persistence`, UI (Настройки → ИИ) |
| `~/.local/share/asiba/history.db` | метрики, журнал действий | `StorageWriter` |
| `~/.local/state/asiba/ignored.toml` | архив инцидентов | UI |
| `~/.local/state/asiba/audits/<сервер>/` | отчёты ИИ | воркер ИИ |
| `~/.cache/asiba/geo.json`, `countries.json`, `tiles/` | геокеш, страны, тайлы | движок, карта |

## 13. Инструменты разработчика

- `cargo run -p asiba-modules --example dump <модуль>` — снимок модуля с локальной машины.
- `cargo test -p asiba-modules --test live_local -- --ignored` — все модули на localhost; `--test live_files` — файловый модуль.
- `ASIBA_SCREENSHOT=/tmp/shot.png ASIBA_SCREENSHOT_DELAY=10 ASIBA_OPEN=<сервер>[/<вкладка>]|overview|servers|alerts|map|settings ASIBA_WINDOW=940x700 cargo run -p asiba-app` — скриншот страницы и выход (`asiba-ui/src/devtools.rs`).
- `RUST_LOG=debug` — уровень логов `tracing` (по умолчанию `info`, вывод в stderr).
