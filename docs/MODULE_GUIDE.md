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

## Реестр команд по модулям

### system
`hostname`, `cat /etc/os-release`, `uname -r`, `uname -m`, `cat /proc/uptime`, `cat /proc/loadavg`, `grep -m1 'model name' /proc/cpuinfo`, `grep -c '^processor' /proc/cpuinfo`, `grep -E '^(MemTotal|SwapTotal):' /proc/meminfo`, `systemd-detect-virt`, `cat /etc/timezone || timedatectl show -p Timezone --value`, `date -u +%s`.
