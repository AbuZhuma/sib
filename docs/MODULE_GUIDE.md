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
- Действия — `Module::actions()` (список `ActionSpec` константами модуля: `SPEC_BAN`, `SPEC_KILL`, …) и `Module::perform(transport, &ActionRequest)`. Это единственные команды записи; каждая перечислена в реестре ниже с пометкой **действие**. Цель действия модуль валидирует сам. UI возвращает `ViewAction::Act { spec, request }`, приложение показывает подтверждение, движок выполняет и пишет в журнал (`actions` в SQLite).
- Запросы по требованию — `Module::query(transport, QueryRequest { kind, target })` → `QueryResponse { title, text }`. UI-представление возвращает `ViewAction::Query`, движок выполняет запрос на живой сессии сервера и отдаёт результат в панель «Просмотр». Так сделаны логи контейнера и журнал юнита.
- Модулю, которому нужен sudo, `Transport::sudo_mode()` говорит, есть ли он; `exec_root` при `SudoMode::None` возвращает ошибку. В `detect` отвечай `Partial { missing }`, если без sudo часть данных недоступна.
- Проверки «снаружи» (доступность порта, пинг) делаются из приложения: `CollectContext.host` — адрес, по которому подключились.

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
`cat /proc/[0-9]*/stat`, чтение `/proc/[pid]/cmdline` и `/proc/[pid]/io` циклом `for`, `ps -eo pid=,user=`, `getconf CLK_TCK`, `getconf PAGESIZE`, `cat /proc/uptime`, `grep MemTotal /proc/meminfo`. **Действия**: `kill -TERM <pid>`, `kill -KILL <pid>` (через sudo, если настроен).

### Пинг (не модуль)
Движок сам раз в 5 с делает TCP-connect на `host:port` SSH с локальной машины и пишет `ping.rtt_ms`. Команд на сервере не выполняет.

### services
`systemctl list-units --type=service --all --plain --no-legend --no-pager`, `systemctl show '*.service' -p Id -p NRestarts -p MainPID -p ActiveEnterTimestamp -p FragmentPath -p WorkingDirectory -p Result`, `systemctl list-timers --all --no-legend -o json`. Детект: `command -v systemctl`. Запрос `journal`: `journalctl -u <unit> -n 200 --no-pager -o short-iso`. **Действия** (root): `systemctl restart|start|stop <unit>`.

### docker
Бинарник: `docker`, иначе `podman`. `version --format`, `ps -a --format <tab-template>`, `stats --no-stream --format`, `images --format`, `inspect --format` по всем контейнерам (RestartCount, Health, RestartPolicy, ExitCode, StartedAt, compose-labels), `volume ls -q | wc -l`, `network ls --format`. Детект: `ps -q` (код ≠ 0 — нет доступа к сокету). Запрос `logs`: `logs --tail 300 -t <container>`. **Действия**: `docker restart|start|stop <container>`.

### ports
`ss -tulpnH`, `ss -Htan state established`. Через sudo (если настроен): `ufw status`, `firewall-cmd --list-all`, `nft list ruleset`, `iptables -S INPUT`. Доступность снаружи: TCP-connect из приложения на публичные порты (≤64, таймаут 1.5 с), повтор не чаще раза в 5 минут при неизменном наборе.

### logs
Первый сбор: `journalctl -p warning -o json --no-pager -q --since -1h -n 300`, далее `--after-cursor=<cursor последней записи>`. Детект: `command -v journalctl`, пробный `journalctl -q -n 1 --system`.

### users
`who`, `last -F -n 30 -w`, `getent passwd`, `getent group sudo wheel admin`, подсчёт строк в `/root/.ssh/authorized_keys` и `/home/*/.ssh/authorized_keys`.

### updates
Детект: `apt-get`/`dnf`/`yum`/`pacman`/`zypper`/`apk`. `apt-get -s upgrade | grep ^Inst`, `dnf -q check-update`, `yum -q check-update`, `pacman -Qu`, `zypper -q lu`, `apk version -l '<'`; перезагрузка: `/var/run/reboot-required`, `needs-restarting -r`.

### projects
`find` по `/opt /srv /var/www /home/* /root /app /docker /data` (глубина 3) маркеров `.git`, `compose*.yml`, `package.json`, `Cargo.toml`, `pyproject.toml`, `go.mod`, `Dockerfile`, `ecosystem.config.js`; `git -C <repo> rev-parse --abbrev-ref HEAD`, `git log -1`, `git status --porcelain | wc -l`; `systemctl show '*.service' -p Id -p WorkingDirectory -p MainPID -p ActiveState`; `readlink /proc/[pid]/cwd` + `/proc/[pid]/comm`; `docker inspect --format` (имя, compose-проект, working_dir); `ss -tlnpH`. Выполняется через sudo, если он настроен (cwd чужих процессов).

### security
Выполняется через sudo, если он настроен (иначе `Partial`). `id -un`, `systemctl is-active` для `firewalld ufw nftables iptables netfilter-persistent fail2ban`, `command -v fail2ban-client nft iptables ufw`, `journalctl -q -o short-iso --since -24h -t sshd -t sshd-session -n 2000` (без journald — `grep sshd /var/log/auth.log /var/log/secure`), то же для `-t sudo -n 500`, `fail2ban-client status` и `fail2ban-client status <jail>` по каждому джейлу, `sshd -T` (fallback — `grep` по `/etc/ssh/sshd_config` и `sshd_config.d/*.conf`), `sha256sum /etc/passwd /etc/group /etc/sudoers /etc/sudoers.d/*`, `nft list set inet asiba bans|bans6`, `iptables -S ASIBA`, `ip6tables -S ASIBA`, `ufw status | grep DENY`.
**Действия** (root): `fail2ban-client set <jail> banip|unbanip <ip>`; `nft add table inet asiba` + set `bans`/`bans6` (`flags timeout`) + chain `input` (hook input, priority -10) + rule `ip saddr @bans drop`, затем `nft add element inet asiba bans '{ <ip> timeout <срок> }'` / `nft delete element …`; `iptables -N ASIBA`, `iptables -I INPUT -j ASIBA`, `iptables -A|-D ASIBA -s <ip> -j DROP` (и `ip6tables`); `ufw insert 1 deny from <ip>` / `ufw delete deny from <ip>`.
