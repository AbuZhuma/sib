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
- Команды модулей только читают, полный список ниже. Записывают лишь действия: бан, kill, restart, stop и правка файлов. Каждое проходит через диалог подтверждения и попадает в журнал.
- Приложение не логирует команды с секретами.
- Без включённого ИИ наружу уходят только запросы к `ip-api.com` для карты и стран адресов. Выключаются в Настройки → Общие.

## Какие команды выполняются на сервере

Полный список того, что программа запускает по SSH. Всё остальное она не трогает. Команды, помеченные как **действия**, выполняются только после диалога подтверждения и записываются в журнал; все прочие читают.

Пинг команд на сервере не выполняет: приложение раз в 5 секунд само делает TCP-connect на порт SSH со своей машины.

**system**
`hostname`, `cat /etc/os-release`, `uname -r`, `uname -m`, `cat /proc/uptime`, `cat /proc/loadavg`, `grep -m1 'model name' /proc/cpuinfo`, `grep -c '^processor' /proc/cpuinfo`, `grep -E '^(MemTotal|SwapTotal):' /proc/meminfo`, `systemd-detect-virt`, `cat /etc/timezone || timedatectl show -p Timezone --value`, `date -u +%s`.

**cpu**
`cat /proc/stat`, `cat /proc/pressure/cpu`, `grep 'cpu MHz' /proc/cpuinfo`, чтение `/sys/class/thermal/thermal_zone*/{type,temp}` и `/sys/class/hwmon/hwmon*/{name,temp1_input}`.

**memory**
`cat /proc/meminfo`, `cat /proc/pressure/memory`, `grep -E '^(oom_kill|pswpin|pswpout) ' /proc/vmstat`.

**disk**
`df -P -B1 -x tmpfs -x devtmpfs -x squashfs -x overlay -x efivarfs -x fuse.portal`, то же с `-i` для inode, `cat /proc/diskstats`, `cat /proc/pressure/io`. Проверка доступности: `command -v df`.

**network**
`cat /proc/net/dev`, `ip -o addr`, `ip -o link`, чтение `/sys/class/net/*/{operstate,speed}`, `ss -Htan | awk '{print $1}' | sort | uniq -c`, `ip -o -4 route show default`.

**processes**
`cat /proc/[0-9]*/stat`, чтение `/proc/<pid>/cmdline` и `/proc/<pid>/io`, `ps -eo pid=,user=`, `getconf CLK_TCK`, `getconf PAGESIZE`, `cat /proc/uptime`, `grep MemTotal /proc/meminfo`.
**Действия** (через sudo, если настроен): `kill -TERM <pid>`, `kill -KILL <pid>`.

**services**
`systemctl list-units --type=service --all --plain --no-legend --no-pager`, `TZ=UTC systemctl show '*.service' -p Id -p NRestarts -p MainPID -p ActiveEnterTimestamp -p FragmentPath -p WorkingDirectory -p Result`, `TZ=UTC systemctl list-timers --all --no-legend -o json`. По запросу: `journalctl -u <unit> -n 200 --no-pager -o short-iso`. Проверка доступности: `command -v systemctl`.
**Действия** (root): `systemctl restart|start|stop <unit>`.

**docker**
Бинарник `docker`, иначе `podman`: `version --format`, `ps -a --format`, `stats --no-stream --format`, `images --format`, `inspect --format` по всем контейнерам, `volume ls -q | wc -l`, `network ls --format`. По запросу: `logs --tail 300 -t <container>`. Проверка доступности: `ps -q`.
**Действия**: `restart|start|stop <container>`.

**ports**
`ss -tulpnH`, `ss -Htan state established`. Через sudo, если настроен: `ufw status`, `firewall-cmd --list-all`, `nft list ruleset`, `iptables -S INPUT`. Доступность портов снаружи приложение проверяет со своей машины TCP-connect'ом, не более 64 портов, таймаут 1.5 секунды, не чаще раза в 5 минут.

**logs**
`journalctl -p warning -o json --no-pager -q --since -1h -n 300`, дальше то же с `--after-cursor=<курсор последней записи>`. Подгрузка истории при прокрутке: то же с `--until=@<время самой старой записи>`. Проверка доступности: `command -v journalctl` и пробный `journalctl -q -n 1 --system`.

**users**
`who`, `last -F -n 30 -w`, `getent passwd`, `getent group sudo wheel admin`, подсчёт строк в `/root/.ssh/authorized_keys` и `/home/*/.ssh/authorized_keys`. По запросу «активность пользователя» (через sudo, если настроен): `getent passwd <user>`, `tail -n 300` его `.bash_history` и `.zsh_history`, `journalctl -t sudo -o short-iso --since -30d`, `journalctl _UID=<uid> -n 200 -o short-iso`. История команд не сохраняется на диск, только показывается.

**updates**
Проверка доступности: `apt-get`, `dnf`, `yum`, `pacman`, `zypper` или `apk`. Сбор: `apt-get -s upgrade | grep ^Inst`, `dnf -q -C check-update` (только по кешу метаданных, полный запрос лишь если кеша нет), `yum -q check-update`, `pacman -Qu`, `zypper -q lu`, `apk version -l '<'`. Перезагрузка: `/var/run/reboot-required`, `needs-restarting -r`.

**security**
Выполняется через sudo, если он настроен. Каждый цикл: `id -un`, `systemctl is-active` для `firewalld ufw nftables iptables netfilter-persistent fail2ban auditd unattended-upgrades dnf-automatic.timer`, `command -v fail2ban-client nft iptables ufw`, `journalctl -q -o short-iso --since -24h -t sshd -t sshd-session -n 2000` (без journald `grep sshd /var/log/auth.log /var/log/secure`), то же для `-t sudo -n 500`, `fail2ban-client status` и `fail2ban-client status <jail>` по каждому джейлу, `nft list set inet sib bans|bans6`, `iptables -S SIB`, `ip6tables -S SIB`, `ufw status | grep DENY`.
Раз в 5 минут: `sshd -T` (если недоступен, `grep` по `/etc/ssh/sshd_config` и `sshd_config.d/*.conf`), `sha256sum /etc/passwd /etc/group /etc/sudoers /etc/sudoers.d/*`, `sysctl` по ключам защищённости, `getenforce`, `aa-status --enabled`, `timedatectl show -p NTPSynchronized --value`, `awk` по `/etc/passwd` и `/etc/shadow`, `grep -r NOPASSWD /etc/sudoers /etc/sudoers.d`, `find` по `authorized_keys` с правами `go+w`, `find /etc -maxdepth 2 -type f -perm -o+w`, `ss -tlnH` по типовым портам баз и панелей.
**Действия** (root): `fail2ban-client set <jail> banip|unbanip <ip>`; для nftables создание таблицы `inet sib` с сетами `bans` и `bans6`, цепочкой `input` и правилом `ip saddr @bans drop`, затем `nft add element` и `nft delete element`; для iptables `-N SIB`, `-I INPUT -j SIB`, `-A|-D SIB -s <ip> -j DROP` (и то же для ip6tables); `ufw insert 1 deny from <ip>` и `ufw delete deny from <ip>`.

**anomalies**
`ss -Htan`, `cat /proc/net/netstat`, `cat /proc/net/snmp`, `cat /proc/net/dev`, `cat /proc/sys/net/netfilter/nf_conntrack_count` и `nf_conntrack_max`. Проверка доступности: `command -v ss && test -r /proc/net/snmp`.

**deploy**
Проверка доступности: `command -v docker || command -v podman`, `systemctl list-units 'deploy*'`, `pgrep -f '[R]unner.Listener|[g]itlab-runner run'`, чтение логов из настройки `[modules.deploy] logs`. Сбор: `docker events --since <unix> --until <unix> --format`, `TZ=UTC systemctl show 'deploy*' -p Id -p ActiveState -p SubState -p Result -p ExecMainStartTimestamp -p ExecMainExitTimestamp -p ExecMainStatus`, `journalctl -u <unit> -n 60 -o short-iso` по каждому юниту `deploy*`, `stat -c %Y` и `tail -n 400` по каждому логу из настройки, `pgrep -f` раннера с `readlink /proc/<pid>/cwd` и `tail -n 40` его последнего лога.

**git**
Проверка доступности: `git --version` и поиск первого `.git` в `/opt /srv /var/www /home/* /root /app /docker /data` (`find -maxdepth 3 -name .git -type d`). Сбор по каждому репозиторию с `GIT_OPTIONAL_LOCKS=0`: `rev-parse --abbrev-ref HEAD`, `rev-parse HEAD`, `remote get-url origin`, `rev-list --left-right --count 'HEAD...@{upstream}'`, `status --porcelain | head -200`, `stash list | wc -l`, `branch --format='%(refname:short)' | head -50`, `tag --sort=-creatordate | head -20`, `log -100`. По запросу истории: `log -2000`. В репозиторий ничего не пишется, `status` с `GIT_OPTIONAL_LOCKS=0` не трогает индекс.

**gpu**
Проверка доступности: `command -v nvidia-smi || ls /sys/class/drm/card*/device/gpu_busy_percent`. Сбор: `nvidia-smi --query-gpu=...` и `nvidia-smi --query-compute-apps=...`; для AMD чтение `/sys/class/drm/card*/device/{gpu_busy_percent,mem_info_vram_used,mem_info_vram_total,product_name}` и `hwmon/hwmon*/{temp1_input,power1_average}`.

**files**
По расписанию ничего не собирает, только по запросу из файлового менеджера. Проверка доступности: `find / -maxdepth 0 -printf ''` и `command -v stat`. Список папки: `find <dir> -mindepth 1 -maxdepth 1 -printf ... | head -n 1000`. Поиск: `timeout 15 nice -n 19 find / \( -path /proc -o -path /sys -o -path /dev -o -path /run \) -prune -o -iname '*<шаблон>*' | head -n 200`. Чтение файла: `stat -c %s`, проверка прав, лимит 200 000 байт, проба на двоичное содержимое, затем `cat`. Всё через sudo, если он настроен.
**Действия**: запись через временный файл рядом с сохранением прав и владельца и `mv -f`, `mkdir`, создание пустого файла, `mv`, `cp -a`, `chmod`, `chown`, `rm -rf`. Пути только абсолютные и без `..`, всё экранируется. Запись в `/etc`, `/boot`, `/usr`, `/bin`, `/sbin` и `/lib` требует ввести имя сервера.

**checks**
Ваши собственные проверки. Проверка доступности: `command -v timeout`. Если проверок нет, модуль не выполняет ни одной команды. Иначе один вызов на все проверки без root и один через sudo, в каждом по фрагменту на проверку: `printf '%s' '<скрипт>' | timeout 20 sh 2>&1`. Скрипт передаётся аргументом `printf` и во временные файлы на сервере не сохраняется.

## Для разработки

- `cargo run -p sib-modules --example dump <модуль>` снимает данные модуля с локальной машины.
- `cargo test -p sib-modules --test live_local -- --ignored` прогоняет все модули на localhost.
- `SIB_SCREENSHOT=/tmp/shot.png SIB_SCREENSHOT_DELAY=10 SIB_OPEN=<сервер>[/<вкладка>]|overview|servers|alerts|map|settings SIB_WINDOW=940x700 cargo run -p sib-app` снимает экран нужной страницы и выходит.
