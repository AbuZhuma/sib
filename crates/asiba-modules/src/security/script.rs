use super::hardening;
use crate::common::sections;

const SSH_LOG: &str = "if command -v journalctl >/dev/null; then journalctl -q --no-pager -o short-iso --since -24h -t sshd -t sshd-session -n 2000; else grep -h 'sshd' /var/log/auth.log /var/log/secure | tail -n 2000; fi";
const SUDO_LOG: &str = "if command -v journalctl >/dev/null; then journalctl -q --no-pager -o short-iso --since -24h -t sudo -n 500; else grep -h 'sudo' /var/log/auth.log /var/log/secure | tail -n 500; fi";
const UNITS: &str = "for s in firewalld ufw nftables iptables netfilter-persistent fail2ban auditd unattended-upgrades dnf-automatic.timer dnf-automatic-install.timer; do printf '%s %s\\n' \"$s\" \"$(systemctl is-active \"$s\" 2>/dev/null)\"; done";
const FAIL2BAN: &str = "fail2ban-client status | sed -n 's/.*Jail list:[[:space:]]*//p' | tr ',' '\\n' | while read -r j; do [ -n \"$j\" ] && echo \"@@ $j\" && fail2ban-client status \"$j\"; done";
const TOOLS: &str = "for t in fail2ban-client nft iptables ufw; do command -v \"$t\" >/dev/null && echo \"$t\"; done";
const SSHD: &str = "sshd -T 2>/dev/null | grep -iE '^(passwordauthentication|permitrootlogin|pubkeyauthentication|port|maxauthtries|permitemptypasswords|x11forwarding|logingracetime|clientaliveinterval|allowtcpforwarding|usepam|maxstartups) ' || grep -hiE '^[[:space:]]*(PasswordAuthentication|PermitRootLogin|PubkeyAuthentication|Port|MaxAuthTries|PermitEmptyPasswords|X11Forwarding|LoginGraceTime|ClientAliveInterval|AllowTcpForwarding|UsePAM|MaxStartups)[[:space:]]' /etc/ssh/sshd_config /etc/ssh/sshd_config.d/*.conf";
const HASHES: &str = "sha256sum /etc/passwd /etc/group /etc/sudoers /etc/sudoers.d/*";
const MAC: &str = "getenforce 2>/dev/null; aa-status --enabled 2>/dev/null && echo apparmor";
const NTP: &str = "timedatectl show -p NTPSynchronized --value";
const UID0: &str = "awk -F: '$3==0 && $1!=\"root\"{print $1}' /etc/passwd";
const SHADOW: &str = "if [ -r /etc/shadow ]; then awk -F: '($2==\"\"){print $1}' /etc/shadow; else echo '@@unreadable'; fi";
const NOPASSWD: &str =
    "[ -r /etc/sudoers ] && grep -rhs NOPASSWD /etc/sudoers /etc/sudoers.d | grep -vc '^#'";
const KEY_PERMISSIONS: &str = "[ \"$(id -u)\" = 0 ] || echo '@@unreadable'; find /root/.ssh /home/*/.ssh -name authorized_keys -perm /go+w 2>/dev/null";
const WORLD_WRITABLE: &str = "find /etc -maxdepth 2 -type f -perm -o+w 2>/dev/null | head -5";
const RISKY_PORTS: &str =
    "ss -tlnH | awk '{print $4}' | grep -E ':(21|23|512|513|514|2375|2376|6379|27017|9200)$'";

const FAST_PARTS: [(&str, &str); 8] = [
    ("whoami", "id -un"),
    ("units", UNITS),
    ("ssh", SSH_LOG),
    ("sudo", SUDO_LOG),
    ("fail2ban", FAIL2BAN),
    (
        "nftbans",
        "nft list set inet asiba bans; nft list set inet asiba bans6",
    ),
    ("iptbans", "iptables -S ASIBA; ip6tables -S ASIBA"),
    ("ufwbans", "ufw status | grep DENY"),
];

const SLOW_PARTS: [(&str, &str); 11] = [
    ("tools", TOOLS),
    ("sshd", SSHD),
    ("hashes", HASHES),
    ("mac", MAC),
    ("ntp", NTP),
    ("uid0", UID0),
    ("shadow", SHADOW),
    ("nopasswd", NOPASSWD),
    ("keyperms", KEY_PERMISSIONS),
    ("wwfiles", WORLD_WRITABLE),
    ("risky", RISKY_PORTS),
];

pub fn build(include_slow: bool) -> String {
    if !include_slow {
        return sections::script(&FAST_PARTS);
    }
    let sysctl = format!("sysctl {}", hardening::SYSCTL_KEYS.join(" "));
    let mut parts: Vec<(&str, &str)> = FAST_PARTS.to_vec();
    parts.extend(SLOW_PARTS);
    parts.push(("sysctl", sysctl.as_str()));
    sections::script(&parts)
}
