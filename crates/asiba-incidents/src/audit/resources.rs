use asiba_core::ServerState;
use asiba_modules::disk::{self, DiskSnapshot};
use asiba_modules::memory::{self, MemorySnapshot};
use asiba_modules::processes::{self, ProcessSnapshot};
use asiba_modules::system::{self, SystemInfo};
use asiba_modules::{cpu, gpu};
use chrono::Duration;

use super::{Area, AuditCheck};

const CPU_WARN_PCT: f64 = 70.0;
const CPU_FAIL_PCT: f64 = 90.0;
const LOAD_WARN_RATIO: f64 = 1.0;
const LOAD_FAIL_RATIO: f64 = 2.0;
const MEMORY_WARN_PCT: f64 = 80.0;
const MEMORY_FAIL_PCT: f64 = 92.0;
const SWAP_WARN_PCT: f64 = 50.0;
const SWAP_FAIL_PCT: f64 = 80.0;
const DISK_WARN_PCT: f64 = 80.0;
const DISK_FAIL_PCT: f64 = 90.0;
const INODES_WARN_PCT: f64 = 80.0;
const INODES_FAIL_PCT: f64 = 90.0;
const ZOMBIES_WARN: usize = 1;
const ZOMBIES_FAIL: usize = 20;
const GPU_TEMP_WARN: f64 = 80.0;
const GPU_TEMP_FAIL: f64 = 90.0;

const ADVICE_CPU: &str = "Найдите процессы-потребители на вкладке «Процессы»; если нагрузка постоянная - добавьте ядер или разнесите сервисы.";
const ADVICE_LOAD: &str = "Load выше числа ядер означает очередь на CPU или ожидание диска - смотрите «Процессы» и I/O дисков.";
const ADVICE_MEMORY: &str = "Проверьте самые прожорливые процессы и контейнеры; добавьте памяти или ограничьте лимиты контейнеров.";
const ADVICE_SWAP: &str =
    "Активный swap замедляет всё - уменьшите потребление памяти или увеличьте RAM.";
const ADVICE_OOM: &str = "Ядро убивало процессы из-за нехватки памяти - найдите виновника в журнале (dmesg, journalctl -k) и ограничьте его.";
const ADVICE_DISK: &str = "Освободите место: журналы (journalctl --vacuum-size), кэш пакетов, старые образы Docker (docker system prune).";
const ADVICE_INODES: &str = "Много мелких файлов исчерпали inode - найдите каталог с тысячами файлов (кэши, сессии, почта).";
const ADVICE_ZOMBIES: &str =
    "Зомби копятся, когда родитель не читает статус потомков - перезапустите родительский процесс.";
const ADVICE_GPU: &str = "Проверьте охлаждение и нагрузку на GPU; при перегреве снижается частота.";

pub fn checks(server: &ServerState) -> Vec<AuditCheck> {
    let mut checks = vec![cpu_hour(server)];
    if let Some(info) = server.data::<SystemInfo>(system::ID) {
        checks.push(load(info));
    }
    if let Some(snapshot) = server.data::<MemorySnapshot>(memory::ID) {
        checks.extend(memory_checks(snapshot));
    }
    if let Some(snapshot) = server.data::<DiskSnapshot>(disk::ID) {
        checks.extend(disk_checks(snapshot));
    }
    if let Some(snapshot) = server.data::<ProcessSnapshot>(processes::ID) {
        checks.push(zombies(snapshot));
    }
    if let Some(snapshot) = server.data::<gpu::GpuSnapshot>(gpu::ID) {
        checks.extend(gpu_checks(snapshot));
    }
    checks
}

fn cpu_hour(server: &ServerState) -> AuditCheck {
    let check = AuditCheck::new(
        Area::Resources,
        "CPU не перегружен (среднее за час)",
        ADVICE_CPU,
    );
    let Some(series) = server.series.get(cpu::KEY_TOTAL) else {
        return check;
    };
    let points: Vec<f64> = series.since(Duration::hours(1)).map(|p| p.value).collect();
    if points.is_empty() {
        return check;
    }
    let average = points.iter().sum::<f64>() / points.len() as f64;
    check.graded(
        average >= CPU_FAIL_PCT,
        average >= CPU_WARN_PCT,
        format!("среднее {average:.0}% по {} точкам", points.len()),
    )
}

fn load(info: &SystemInfo) -> AuditCheck {
    let cores = info.cpu_cores.max(1) as f64;
    let ratio = info.load.five / cores;
    AuditCheck::new(
        Area::Resources,
        "Load average в пределах числа ядер",
        ADVICE_LOAD,
    )
    .graded(
        ratio >= LOAD_FAIL_RATIO,
        ratio >= LOAD_WARN_RATIO,
        format!("load5 {:.2} на {} ядер", info.load.five, info.cpu_cores),
    )
}

fn memory_checks(snapshot: &MemorySnapshot) -> Vec<AuditCheck> {
    let used = snapshot.used_pct();
    let mut checks = vec![
        AuditCheck::new(Area::Resources, "Память не исчерпана", ADVICE_MEMORY).graded(
            used >= MEMORY_FAIL_PCT,
            used >= MEMORY_WARN_PCT,
            format!("занято {used:.0}%"),
        ),
        AuditCheck::new(Area::Resources, "OOM killer не срабатывал", ADVICE_OOM).graded(
            false,
            snapshot.oom_kills > 0,
            format!("{} срабатываний с загрузки", snapshot.oom_kills),
        ),
    ];
    if snapshot.swap_total_bytes > 0 {
        let swap = snapshot.swap_used_pct();
        checks.push(
            AuditCheck::new(Area::Resources, "Swap почти не используется", ADVICE_SWAP).graded(
                swap >= SWAP_FAIL_PCT,
                swap >= SWAP_WARN_PCT,
                format!("занято {swap:.0}%"),
            ),
        );
    }
    checks
}

fn disk_checks(snapshot: &DiskSnapshot) -> Vec<AuditCheck> {
    let mut checks = Vec::new();
    for fs in &snapshot.filesystems {
        let used = fs.used_pct();
        checks.push(
            AuditCheck::new(
                Area::Resources,
                format!("Место на {}", fs.mount),
                ADVICE_DISK,
            )
            .graded(
                used >= DISK_FAIL_PCT,
                used >= DISK_WARN_PCT,
                format!(
                    "занято {used:.0}%, свободно {} MiB",
                    fs.available_bytes / 1024 / 1024
                ),
            ),
        );
        if fs.inodes_total == 0 {
            continue;
        }
        let inodes = fs.inodes_used_pct();
        checks.push(
            AuditCheck::new(
                Area::Resources,
                format!("Inode на {}", fs.mount),
                ADVICE_INODES,
            )
            .graded(
                inodes >= INODES_FAIL_PCT,
                inodes >= INODES_WARN_PCT,
                format!("занято {inodes:.0}%"),
            ),
        );
    }
    checks
}

fn zombies(snapshot: &ProcessSnapshot) -> AuditCheck {
    let zombies = snapshot.zombie_count();
    AuditCheck::new(Area::Resources, "Нет зомби-процессов", ADVICE_ZOMBIES).graded(
        zombies >= ZOMBIES_FAIL,
        zombies >= ZOMBIES_WARN,
        format!("{zombies} зомби из {} процессов", snapshot.processes.len()),
    )
}

fn gpu_checks(snapshot: &gpu::GpuSnapshot) -> Vec<AuditCheck> {
    snapshot
        .gpus
        .iter()
        .filter_map(|card| {
            let temperature = card.temperature_c?;
            Some(
                AuditCheck::new(
                    Area::Resources,
                    format!("GPU {} не перегрет", card.index),
                    ADVICE_GPU,
                )
                .graded(
                    temperature >= GPU_TEMP_FAIL,
                    temperature >= GPU_TEMP_WARN,
                    format!("{temperature:.0} °C, память {:.0}%", card.memory_pct()),
                ),
            )
        })
        .collect()
}
