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
| `asiba-config` | пути XDG, `config.toml`, `servers/<name>.toml`, keyring, раскладка виджетов | сеть, UI |
| `asiba-modules` | сборщики данных и их парсеры | как и когда их вызывают |
| `asiba-storage` | SQLite: история метрик, даунсэмплинг, поток записи | модули, UI |
| `asiba-incidents` | детекторы инцидентов (`Detector`) и сверка с состоянием (`reconcile`) | docgen, движок |
| `asiba-ai` | бэкенд `Backend` (`Gemini` через REST API), сборка контекста с бюджетом токенов, плейбуки инцидентов, промпт | движок |
| `asiba-docgen` | секции данных сервера; рендер `servers/<name>.md` (человек) и `servers/<name>.llm.md` (модель) | движок, UI |
| `asiba-alerts` | встроенные правила, оценка правил и базовой линии над `AppState`, уведомления на рабочий стол | транспорт, UI |
| `asiba-engine` | воркеры серверов, задачи сбора по модулям, пинг, переподключение, геолокация, алерты, действия, команды от UI | egui |
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

## Запросы по требованию

UI (`ModuleView::page`) → `ViewAction::Query` → `Action::Query` → `Command::Query { token, server, module, request }` → движок берёт транспорт сервера из слота воркера → `Module::query` → `EngineEvent::QueryFinished` → панель «Просмотр» на странице сервера.

## Действия

UI → `ViewAction::Act { spec, request }` → `Action::AskPerform` → диалог подтверждения (`Danger::High` или production — с вводом имени сервера) → `Command::Perform` → `Module::perform` на транспорте сервера → `ActionRecord` в SQLite (`actions`), в `AppState.actions` и `EngineEvent::ActionFinished` (уведомление в статусбаре). Журнал — в настройках.

## Алерты

Задача движка раз в 5 с: `Evaluator::evaluate(&mut AppState)` — правила (встроенные из `asiba-alerts::builtin_rules` + пользовательские из `config.toml`) по последним значениям серий и виртуальной метрике `connection.offline`, плюс базовая линия EWMA по ключевым метрикам. Поднятые алерты — в `AppState.alerts`, warning/critical уходят на рабочий стол. `Command::AcknowledgeAlert` / `MuteAlert` меняют запись; `SetAlertSettings` обновляет правила через `watch`.

## Геолокация и карта

При старте воркера `geo::resolve_server`: ручные координаты → кеш `geo.json` → `ip-api.com`. Результат — `ServerState.location`; положение локальной машины — `AppState.self_location`. UI рисует карту компонентом `MapState` (`walkers`, OSM-тайлы с кешем в `~/.cache/asiba/tiles`), маркеры и линии пинга — плагином `Markers`.

## Файл сервера

`asiba-docgen` описывает данные сервера как набор секций (`Section` с `SectionId`, по одному файлу в `sections/`): каждая секция умеет `human` (русский markdown) и `llm` (английский, `key: value`, ограниченные списки). `render_human` и `render_llm` собирают все доступные секции, `render_llm_sections` — выбранные (для контекста модели). Секция `findings` показывает выводы детекторов `asiba-incidents` и идёт первой. `DocWriter` в воркере после каждого цикла любого модуля, не чаще раза в 10 с и только при изменении тела, пишет `servers/<name>.md` (сохраняя блок `<!-- notes:start -->…<!-- notes:end -->`) и `servers/<name>.llm.md`.

## Данные для графиков

`Snapshot.samples` → `ServerState.series[key]` (кольцо 900 точек) → `TimeSeriesPlot`/`sparkline` в UI. Те же сэмплы уходят в `StorageWriter` → SQLite (`samples` → `samples_1m` → `samples_1h`).

## Модули v1

system, cpu, memory, disk, network, processes, services, docker, ports, logs, users, updates, projects, security, anomalies, deploy, gpu. Расписания: Fast (cpu, memory, disk, network, processes, anomalies, gpu), Normal (services, docker, ports, logs, users, security, deploy), Slow (system, updates, projects). Интервалы настраиваются глобально и на сервер.

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

## Инциденты

`asiba-incidents` — единый источник «негатива»: каждый детектор (`detectors/<name>.rs`, трейт `Detector`) по состоянию сервера возвращает `IncidentDraft { kind, severity, subject, summary, evidence }`. Детекторы: алерты, признаки DDoS, брутфорс SSH, проверки безопасности, упавшие юниты, контейнеры (остановлен при политике перезапуска, unhealthy, рестарт-петля), упавшие деплои, диски, память (OOM, swap), обновления безопасности, ошибки модулей, расхождение часов. `reconcile(&mut AppState, now)` сравнивает черновики (уровень ≥ warning) с активными `AppState.incidents` по `kind + subject`: новые открывает, исчезнувшие закрывает, совпавшие обновляет. Цикл алертов в движке вызывает `reconcile` каждые 5 с, пишет события, шлёт уведомления на рабочий стол и `EngineEvent::IncidentsOpened`. UI показывает активные инциденты на главной и в сводке сервера. Инциденты — точка запуска ЛЛМ-аудита (см. `docs/LLM-PLAN.md`): новый детектор автоматически становится новым триггером.

## ИИ-анализ (Gemini)

`asiba-ai`: `Backend` (реализация `Gemini` — `POST generativelanguage.googleapis.com/v1beta/models/<model>:generateContent`, ключ в заголовке `x-goog-api-key`), `ContextBuilder` (части с приоритетом, бюджет токенов, обрезание по строкам), `playbook(kind)` — какие секции и какая задача для вида инцидента, `queries(incident)` — что дополнительно спросить у сервера (логи контейнера, journal юнита), `SYSTEM_PROMPT` и `build_user` (ответ на русском).

Движок (`engine/src/ai/`): один воркер `AuditWorker` с очередью заданий. `Command::Audit { server, scope, is_auto }` приходит от UI (кнопка «Полный аудит», «разобрать») или из цикла алертов при открытии инцидента (если `AiConfig::is_ready()` и `auto_audit`). Воркер проверяет согласие и ключ, для автозаданий — минимальный уровень и кулдаун по `server:kind:subject`, кладёт `AuditReport { status: Running }` в `AppState.audits`, собирает контекст (`context::prepare`: запросы к серверу через `Module::query`, затем секции по плейбуку через `render_llm_section`), выполняет запрос в `spawn_blocking`, пишет отчёт в `audits_dir/<сервер>/<время>_<scope>.md`, шлёт `EngineEvent::AuditFinished`. Настройки — `AppConfig.ai` (`AiConfig`: `consent`, `api_key`, `model`, лимиты), меняются на лету через `Command::SetAiConfig`. Без `consent = true` ни один запрос наружу не выполняется.
