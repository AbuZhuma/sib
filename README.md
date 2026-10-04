# Sib

Linux server monitoring in one window. Nothing is installed on the server: the app connects over SSH, reads `/proc`, `/sys` and the output of ordinary tools, and writes the history into SQLite and into a markdown file next to the server settings.

## What it does

- **Servers.** Cards and a table, search and filters by status, environment and tags. A server is added by host, user and key. Without a key it uses ssh-agent and `~/.ssh`. Jump hosts and sudo are supported.
- **19 modules.** System, CPU, memory, disks, network, processes, systemd services, Docker and Podman, ports and firewall, journal, users, updates, security, anomalies and DDoS, deploys, git, GPU, file manager, your own checks. A tab shows up only if the module works on that server and has data.
- **Charts** with history in SQLite: raw points for 48 hours, per minute for 30 days, per hour for a year.
- **Actions** with a confirmation dialog and a journal: ban an IP through fail2ban, nftables, iptables or ufw, stop a process, restart or stop a unit or a container. The dangerous ones ask you to type the server name.
- **Alerts.** Ten built-in rules. Any of them can be edited, turned off or reset, and your own rules are added in the same place. Plus a baseline on the key metrics and desktop notifications.
- **Audit.** 77 security and reliability checks with a protection score from A to F. The weight and the on/off state of every check are set per server, and your own checks go in the same list: a command, a script from the form, a file from this machine or a file on the server.
- **Incidents.** One list of everything that is wrong, on the overview page and on the server card.
- **Map.** Servers by geolocation or by coordinates you set, with lines from your machine showing the ping time.
- **AI analysis.** Off by default, turned on with a switch in the settings. Providers: Gemini, OpenAI, Claude and any OpenAI-compatible API by URL (Groq, Mistral, DeepSeek, your own server). Once it is on, the model reviews every incident, analyses server sections on a button or automatically, and runs a full audit. Reports are kept in `~/.local/state/sib/audits/`. While the switch is off, nothing leaves your machine.
- **Pipelines.** A pipeline is a list of shell commands saved once and run on any server with one click: install tools, clone a repository, build, start a long job with `nohup`, bring the results back with `rsync`. Steps run on the server or on this computer, as a user or as root, with a timeout each, stopping at the first failure unless told to continue. Variables are written as `{{name}}` and filled per server; secret ones are asked at run time and never written to disk. Built-in values `{{host}}`, `{{user}}`, `{{port}}`, `{{ssh_target}}` and `{{ssh_args}}` make local `rsync`/`scp` steps work with jump hosts and keys. Output streams live into the server card, every run is kept in the history with per-step exit codes, and a run can be cancelled or repeated. Pipelines are stored as `.toml` files that can be exported and imported.
- **Terminal.** A button opens the system terminal emulator with `ssh` already running, with the port and the jump host filled in. It looks for `$TERMINAL`, `x-terminal-emulator`, ptyxis, gnome-terminal, konsole, tilix, wezterm, xfce4-terminal, alacritty, kitty, foot, xterm.
- **Server file.** `<name>.md` repeats everything the program shows and is updated after each collect cycle. The block between `<!-- notes:start -->` and `<!-- notes:end -->` is yours, the program never overwrites it. Next to it is `<name>.llm.md`: the same data in a compact form for a language model.

## Install

A build for x86_64 Linux is on the [Releases](https://github.com/AbuZhuma/sib/releases) page:

```
curl -L https://github.com/AbuZhuma/sib/releases/latest/download/sib-x86_64-linux.tar.gz | tar xz
./sib
```

You need a desktop with a Secret Service for the keyring (GNOME Keyring, KWallet) and the usual window libraries: `libxkbcommon`, `libwayland-client` or `libX11`, `libdbus-1`, and an OpenGL driver. SQLite is built into the binary.

## Build from source

You need Rust 1.88 or newer and the headers of the same libraries. Fedora: `libxkbcommon-devel wayland-devel dbus-devel`. Debian and Ubuntu: `libxkbcommon-dev libwayland-dev libdbus-1-dev`.

```
cargo run -p sib-app --release
```

Checks before a commit. CI runs the same steps in `.github/workflows/ci.yml`, plus `cargo deny check` against `deny.toml`:

```
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --workspace
```

## Where things are kept

| Path | Content |
|---|---|
| `~/.config/sib/config.toml` | theme, intervals, retention, alert rules, notifications, AI settings without the key |
| `~/.config/sib/servers/<name>.toml` | server settings without secrets |
| `~/.config/sib/servers/<name>.md` | the server file for people, updated automatically |
| `~/.config/sib/servers/<name>.llm.md` | the server file for a language model |
| `~/.config/sib/pipelines/<id>.toml` | saved pipelines: steps and variables, shared by all servers |
| `~/.local/state/sib/audits/<name>/` | AI reports |
| `~/.local/state/sib/ignored.toml` | incidents you archived |
| `~/.local/share/sib/history.db` | metric history, the action journal and pipeline runs |
| `~/.cache/sib/` | geolocation, address countries, map tiles |
| system keyring | passwords, passphrases, the sudo password, the AI provider API key |

## Server file `servers/<name>.toml`

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

[jump]                         # optional
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

[location]                     # optional, otherwise looked up by IP
lat = 52.52
lon = 13.40
label = "Berlin, Hetzner"

[modules.deploy]               # module settings
logs = "/var/log/deploy.log,/srv/shop/deploy.log"

[modules.processes]
interval = "5"                 # collect interval in seconds for this module
enabled = "true"               # "false" turns the module off on this server

[[checks]]                     # your own audit check, edited in the UI
id = "custom:1"
name = "Certificate is not expiring"
kind = "script"                # script | local_file | remote_file
source = "openssl x509 -checkend 604800 -noout -in /etc/ssl/site.pem"
expect = "exit_zero"           # exit_zero | contains | missing
area = "reliability"
weight = "high"                # low | medium | high
advice = "Renew the certificate"

[check_overrides."ssh.port"]   # a built-in check you changed
enabled = false
```

Secrets never go into the file: passwords are typed in the form and stored in the keyring.

## Security

- The host key is checked against `~/.ssh/known_hosts`. An unknown or changed key is shown with its fingerprint and has to be confirmed.
- Module commands only read, the full list is below. Only actions write: ban, kill, restart, stop and file editing. Each of them goes through a confirmation dialog and lands in the journal.
- The app never logs commands that carry secrets.
- Without AI turned on, the only requests that leave your machine are to `ip-api.com`, for the map and for address countries. They are turned off in Settings > General.

## What the app runs on your servers

The full list of what the program runs over SSH. It touches nothing else. Commands marked as **actions** run only after a confirmation dialog and are written to the journal; everything else only reads.

Ping runs no command on the server: the app makes a TCP connect to the SSH port from its own machine every 5 seconds.

**system**
`hostname`, `cat /etc/os-release`, `uname -r`, `uname -m`, `cat /proc/uptime`, `cat /proc/loadavg`, `grep -m1 'model name' /proc/cpuinfo`, `grep -c '^processor' /proc/cpuinfo`, `grep -E '^(MemTotal|SwapTotal):' /proc/meminfo`, `systemd-detect-virt`, `cat /etc/timezone || timedatectl show -p Timezone --value`, `date -u +%s`.

**cpu**
`cat /proc/stat`, `cat /proc/pressure/cpu`, `grep 'cpu MHz' /proc/cpuinfo`, reads of `/sys/class/thermal/thermal_zone*/{type,temp}` and `/sys/class/hwmon/hwmon*/{name,temp1_input}`.

**memory**
`cat /proc/meminfo`, `cat /proc/pressure/memory`, `grep -E '^(oom_kill|pswpin|pswpout) ' /proc/vmstat`.

**disk**
`df -P -B1 -x tmpfs -x devtmpfs -x squashfs -x overlay -x efivarfs -x fuse.portal`, the same with `-i` for inodes, `cat /proc/diskstats`, `cat /proc/pressure/io`. Availability check: `command -v df`.

**network**
`cat /proc/net/dev`, `ip -o addr`, `ip -o link`, reads of `/sys/class/net/*/{operstate,speed}`, `ss -Htan | awk '{print $1}' | sort | uniq -c`, `ip -o -4 route show default`.

**processes**
`cat /proc/[0-9]*/stat`, reads of `/proc/<pid>/cmdline` and `/proc/<pid>/io`, `ps -eo pid=,user=`, `getconf CLK_TCK`, `getconf PAGESIZE`, `cat /proc/uptime`, `grep MemTotal /proc/meminfo`.
**Actions** (through sudo if it is set up): `kill -TERM <pid>`, `kill -KILL <pid>`.

**services**
`systemctl list-units --type=service --all --plain --no-legend --no-pager`, `TZ=UTC systemctl show '*.service' -p Id -p NRestarts -p MainPID -p ActiveEnterTimestamp -p FragmentPath -p WorkingDirectory -p Result`, `TZ=UTC systemctl list-timers --all --no-legend -o json`. On request: `journalctl -u <unit> -n 200 --no-pager -o short-iso`. Availability check: `command -v systemctl`.
**Actions** (root): `systemctl restart|start|stop <unit>`.

**docker**
The `docker` binary, or `podman`: `version --format`, `ps -a --format`, `stats --no-stream --format`, `images --format`, `inspect --format` over all containers, `volume ls -q | wc -l`, `network ls --format`. On request: `logs --tail 300 -t <container>`. Availability check: `ps -q`.
**Actions**: `restart|start|stop <container>`.

**ports**
`ss -tulpnH`, `ss -Htan state established`. Through sudo if it is set up: `ufw status`, `firewall-cmd --list-all`, `nft list ruleset`, `iptables -S INPUT`. Whether a port answers from outside is checked by the app from its own machine with a TCP connect, at most 64 ports, 1.5 second timeout, no more than once every 5 minutes.

**logs**
`journalctl -p warning -o json --no-pager -q --since -1h -n 300`, then the same with `--after-cursor=<cursor of the last entry>`. Loading older entries while scrolling: the same with `--until=@<time of the oldest entry>`. Availability check: `command -v journalctl` and a test `journalctl -q -n 1 --system`.

**users**
`who`, `last -F -n 30 -w`, `getent passwd`, `getent group sudo wheel admin`, a line count of `/root/.ssh/authorized_keys` and `/home/*/.ssh/authorized_keys`. On a user activity request (through sudo if it is set up): `getent passwd <user>`, `tail -n 300` of their `.bash_history` and `.zsh_history`, `journalctl -t sudo -o short-iso --since -30d`, `journalctl _UID=<uid> -n 200 -o short-iso`. The command history is never saved to disk, only shown.

**updates**
Availability check: `apt-get`, `dnf`, `yum`, `pacman`, `zypper` or `apk`. Collect: `apt-get -s upgrade | grep ^Inst`, `dnf -q -C check-update` (metadata cache only, the full query only when there is no cache), `yum -q check-update`, `pacman -Qu`, `zypper -q lu`, `apk version -l '<'`. Reboot: `/var/run/reboot-required`, `needs-restarting -r`.

**security**
Runs through sudo if it is set up. Every cycle: `id -un`, `systemctl is-active` for `firewalld ufw nftables iptables netfilter-persistent fail2ban auditd unattended-upgrades dnf-automatic.timer`, `command -v fail2ban-client nft iptables ufw`, `journalctl -q -o short-iso --since -24h -t sshd -t sshd-session -n 2000` (without journald, `grep sshd /var/log/auth.log /var/log/secure`), the same for `-t sudo -n 500`, `fail2ban-client status` and `fail2ban-client status <jail>` for each jail, `nft list set inet sib bans|bans6`, `iptables -S SIB`, `ip6tables -S SIB`, `ufw status | grep DENY`.
Every 5 minutes: `sshd -T` (if it is not available, `grep` over `/etc/ssh/sshd_config` and `sshd_config.d/*.conf`), `sha256sum /etc/passwd /etc/group /etc/sudoers /etc/sudoers.d/*`, `sysctl` for the hardening keys, `getenforce`, `aa-status --enabled`, `timedatectl show -p NTPSynchronized --value`, `awk` over `/etc/passwd` and `/etc/shadow`, `grep -r NOPASSWD /etc/sudoers /etc/sudoers.d`, `find` for `authorized_keys` with `go+w`, `find /etc -maxdepth 2 -type f -perm -o+w`, `ss -tlnH` for the usual database and panel ports.
**Actions** (root): `fail2ban-client set <jail> banip|unbanip <ip>`; for nftables it creates the table `inet sib` with the sets `bans` and `bans6`, the chain `input` and the rule `ip saddr @bans drop`, then `nft add element` and `nft delete element`; for iptables `-N SIB`, `-I INPUT -j SIB`, `-A|-D SIB -s <ip> -j DROP` (and the same for ip6tables); `ufw insert 1 deny from <ip>` and `ufw delete deny from <ip>`.

**anomalies**
`ss -Htan`, `cat /proc/net/netstat`, `cat /proc/net/snmp`, `cat /proc/net/dev`, `cat /proc/sys/net/netfilter/nf_conntrack_count` and `nf_conntrack_max`. Availability check: `command -v ss && test -r /proc/net/snmp`.

**deploy**
Availability check: `command -v docker || command -v podman`, `systemctl list-units 'deploy*'`, `pgrep -f '[R]unner.Listener|[g]itlab-runner run'`, and reading the logs from the `[modules.deploy] logs` setting. Collect: `docker events --since <unix> --until <unix> --format`, `TZ=UTC systemctl show 'deploy*' -p Id -p ActiveState -p SubState -p Result -p ExecMainStartTimestamp -p ExecMainExitTimestamp -p ExecMainStatus`, `journalctl -u <unit> -n 60 -o short-iso` for each `deploy*` unit, `stat -c %Y` and `tail -n 400` for each log from the setting, `pgrep -f` for the runner with `readlink /proc/<pid>/cwd` and `tail -n 40` of its latest log.

**git**
Availability check: `git --version` and a search for the first `.git` in `/opt /srv /var/www /home/* /root /app /docker /data` (`find -maxdepth 3 -name .git -type d`). Collect, for each repository, with `GIT_OPTIONAL_LOCKS=0`: `rev-parse --abbrev-ref HEAD`, `rev-parse HEAD`, `remote get-url origin`, `rev-list --left-right --count 'HEAD...@{upstream}'`, `status --porcelain | head -200`, `stash list | wc -l`, `branch --format='%(refname:short)' | head -50`, `tag --sort=-creatordate | head -20`, `log -100`. On a history request: `log -2000`. Nothing is written to the repository, and `status` with `GIT_OPTIONAL_LOCKS=0` does not touch the index.

**gpu**
Availability check: `command -v nvidia-smi || ls /sys/class/drm/card*/device/gpu_busy_percent`. Collect: `nvidia-smi --query-gpu=...` and `nvidia-smi --query-compute-apps=...`; for AMD, reads of `/sys/class/drm/card*/device/{gpu_busy_percent,mem_info_vram_used,mem_info_vram_total,product_name}` and `hwmon/hwmon*/{temp1_input,power1_average}`.

**files**
Collects nothing on a schedule, only on request from the file manager. Availability check: `find / -maxdepth 0 -printf ''` and `command -v stat`. Folder listing: `find <dir> -mindepth 1 -maxdepth 1 -printf ... | head -n 1000`. Search: `timeout 15 nice -n 19 find / \( -path /proc -o -path /sys -o -path /dev -o -path /run \) -prune -o -iname '*<pattern>*' | head -n 200`. Reading a file: `stat -c %s`, a permission check, a limit of 200 000 bytes, a test for binary content, then `cat`. All of it through sudo if it is set up.
**Actions**: writing through a temporary file next to the original, keeping the permissions and the owner, then `mv -f`; `mkdir`; creating an empty file; `mv`; `cp -a`; `chmod`; `chown`; `rm -rf`. Paths must be absolute and without `..`, and everything is quoted. Writing into `/etc`, `/boot`, `/usr`, `/bin`, `/sbin` and `/lib` asks you to type the server name.

**checks**
Your own checks. Availability check: `command -v timeout`. With no checks the module runs no command at all. Otherwise one call for all the checks without root and one through sudo, each with a fragment per check: `printf '%s' '<script>' | timeout 20 sh 2>&1`. The script is passed as an argument to `printf` and is never saved to a file on the server.

## For development

- `cargo run -p sib-modules --example dump <module>` takes a snapshot of one module from the local machine.
- `cargo test -p sib-modules --test live_local -- --ignored` runs every module against localhost.
- `SIB_SCREENSHOT=/tmp/shot.png SIB_SCREENSHOT_DELAY=10 SIB_OPEN=<server>[/<tab>]|overview|servers|alerts|map|settings SIB_WINDOW=940x700 cargo run -p sib-app` takes a screenshot of the given page and exits.
