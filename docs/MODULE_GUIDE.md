# Как добавить модуль

## Шаги

1. Папка `crates/asiba-modules/src/<id>/` с файлами:
   - `mod.rs` — `pub const ID: ModuleId = ModuleId("<id>")`, структура модуля, `impl Module`, список частей скрипта.
   - `model.rs` — структуры данных снимка. Только данные и форматирующие методы.
   - `parse.rs` — чистые функции `&str -> Result<T, ModuleError>` и тесты на фикстурах.
2. Фикстуры реального вывода в `crates/asiba-modules/fixtures/<id>/<distro>.txt` — минимум две разные системы.
3. Регистрация в `asiba_modules::default_registry()` — одна строка.
4. Представление в UI: `crates/asiba-ui/src/modules/<id>.rs`, `impl ModuleView`, регистрация в `modules::all()`.
5. Раздел в этом файле с перечнем команд, которые модуль выполняет на сервере.

## Правила

- `detect` обязан вернуть `Unavailable { reason }`, если данных на сервере нет. Проверяй наличие бинарника (`command -v docker`) и прав.
- Сбор — одна команда через `common::sections::script(&[(name, cmd), ...])`, ответ — `Sections::parse`. Не делай десять `exec` подряд.
- Данные модуля не тянут другие модули напрямую. Если нужна связь (порты ↔ процессы), она делается на уровне модели в `projects` или в UI.
- Расписание: `Fast` — только для живых метрик (cpu, memory, network, processes, docker stats); `Normal` — списки и состояния; `Slow` — редко меняющееся.
- Действия (`ActionSpec`) появятся на этапе 4; до этого модули только читают.

## Скорости и дельты

Модуль получает `CollectContext` с предыдущим снимком того же модуля. `context.previous::<Model>()` возвращает `(предыдущие данные, прошедшие секунды)`. Счётчики (`/proc/stat`, `/proc/net/dev`, `/proc/diskstats`) хранятся в модели как есть, скорости считаются через `common::rate::per_second` и кладутся в поле `Option<...>`: первый снимок после подключения скоростей не имеет, UI показывает «—».

## Метрики для истории

`Snapshot::with_samples` — список `Sample { key, value }`. Ключи: `<модуль>.<метрика>` для сводных значений, `<модуль>.<объект>.<метрика>` для объектов (`network.eth0.rx_bps`, `disk.fs./.used_pct`). Константы ключей объявлены в `mod.rs` модуля и используются в UI. Сэмплы попадают в `ServerState.series` (кольцо на 900 точек) и в SQLite.

## Реестр команд по модулям

### system
`hostname`, `cat /etc/os-release`, `uname -r`, `uname -m`, `cat /proc/uptime`, `cat /proc/loadavg`, `grep -m1 'model name' /proc/cpuinfo`, `grep -c '^processor' /proc/cpuinfo`, `grep -E '^(MemTotal|SwapTotal):' /proc/meminfo`, `systemd-detect-virt`, `cat /etc/timezone || timedatectl show -p Timezone --value`, `date -u +%s`.

### cpu
`cat /proc/stat`, `cat /proc/pressure/cpu`, `grep 'cpu MHz' /proc/cpuinfo`, чтение `/sys/class/thermal/thermal_zone*/{type,temp}`, `/sys/class/hwmon/hwmon*/{name,temp1_input}`.

### memory
`cat /proc/meminfo`, `cat /proc/pressure/memory`, `grep -E '^(oom_kill|pswpin|pswpout) ' /proc/vmstat`.

### disk
`df -P -B1 -x tmpfs -x devtmpfs -x squashfs -x overlay -x efivarfs -x fuse.portal`, `df -P -i …`, `cat /proc/diskstats`, `cat /proc/pressure/io`. Детект: `command -v df`.

### network
`cat /proc/net/dev`, `ip -o addr`, `ip -o link`, чтение `/sys/class/net/*/{operstate,speed}`, `ss -Htan | awk '{print $1}' | sort | uniq -c`, `ip -o -4 route show default`.

### processes
`cat /proc/[0-9]*/stat`, чтение `/proc/[pid]/cmdline` и `/proc/[pid]/io` циклом `for`, `ps -eo pid=,user=`, `getconf CLK_TCK`, `getconf PAGESIZE`, `cat /proc/uptime`, `grep MemTotal /proc/meminfo`.

### Пинг (не модуль)
Движок сам раз в 5 с делает TCP-connect на `host:port` SSH с локальной машины и пишет `ping.rtt_ms`. Команд на сервере не выполняет.
