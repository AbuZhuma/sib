# Архитектура

## Крейты и направление зависимостей

```
asiba-core ◄── asiba-transport ◄──┐
     ▲                            │
     ├── asiba-config ◄───────────┼── asiba-engine ◄── asiba-ui ◄── asiba-app
     │                            │        ▲
     └── asiba-modules ◄──────────┘        │
              ▲                            │
              └────────────────────────────┘
```

| Крейт | Отвечает за | Не знает про |
|---|---|---|
| `asiba-core` | доменные типы: `ServerSpec`, `Transport`, `Module`, `Snapshot`, `AppState` | SSH, egui, SQLite, файлы |
| `asiba-transport` | SSH-сессии (`russh`), локальный транспорт, sudo, known_hosts | модули, UI |
| `asiba-config` | пути XDG, `config.toml`, `servers/<name>.toml`, keyring | сеть, UI |
| `asiba-modules` | сборщики данных и их парсеры | как и когда их вызывают |
| `asiba-storage` | SQLite: история метрик, даунсэмплинг, поток записи | модули, UI |
| `asiba-engine` | воркеры серверов, задачи сбора по модулям, пинг, переподключение, команды от UI | egui |
| `asiba-ui` | тема, страницы, виджеты модулей | сеть напрямую |
| `asiba-app` | точка входа, tokio runtime, сборка зависимостей | — |

## Потоки

- **UI-поток** — eframe. Читает `SharedState` (`Arc<RwLock<AppState>>`) на каждом кадре, пишет команды в `mpsc` канал движка, забирает события через `EngineHandle::poll_events`.
- **tokio runtime** — движок и по одной задаче на сервер (`worker::run`). Задачи пишут в `SharedState` и дёргают `RepaintNotifier`, чтобы UI перерисовался.
- Блокирующие операции (keyring, файлы) — в `spawn_blocking`.

## Жизненный цикл сервера

```
Command::AddServer ─► Persistence::save ─► start_worker
                                              │
             ┌────────────────────────────────┘
             ▼
   connect ──► ok ──► detect_all ──► JoinSet: задача на каждый доступный модуль
     │                                    │ каждая: tick(interval) → collect(previous) → state + storage
     │ UnknownHostKey / HostKeyChanged     │ Disconnected → watch-канал
     ▼                                    ▼
   UntrustedHostKey (ждёт TrustHostKey)  Offline ──► backoff 5/10/30/60 с ──► connect
```

- `detect` повторяется каждые 10 минут (перезапуск набора задач).
- Задача модуля останавливается после 3 подряд ошибок сбора до следующего `detect`.
- Модуль с `Unavailable` не планируется и не показывается в UI.
- Параллельно с соединением живёт задача пинга: TCP-connect на SSH-порт раз в 5 с, результат в `ServerState.ping` и серии `ping.rtt_ms`.

## Данные для графиков

`Snapshot.samples` → `ServerState.series[key]` (кольцо 900 точек) → `TimeSeriesPlot`/`sparkline` в UI. Те же сэмплы уходят в `StorageWriter` → SQLite (`samples` → `samples_1m` → `samples_1h`).

## Модуль

Модуль = папка в `asiba-modules/src/<id>/` с `mod.rs` (реализация `Module`), `model.rs` (данные), `parse.rs` (чистые парсеры). Собирает всё одной командой-скриптом через `common::sections::script`, ответ режется по маркерам `###name`. Подробности — `MODULE_GUIDE.md`.

Представление модуля в UI = `asiba-ui/src/modules/<id>.rs`, реализует `ModuleView`, получает `Snapshot` и делает `downcast::<Model>()`.

## Хранение

| Что | Где |
|---|---|
| конфигурация | `~/.config/asiba/config.toml` |
| серверы | `~/.config/asiba/servers/<name>.toml` |
| секреты | keyring, сервис `asiba`, аккаунт `<name>/password` / `<name>/passphrase` / `<name>/sudo` |
| история (этап 2) | `~/.local/share/asiba/history.db` |
