use chrono::Duration;
use sib_modules::disk::{self, DiskSnapshot};
use sib_modules::memory::{self, MemorySnapshot};
use sib_modules::processes::{self, ProcessSnapshot};
use sib_modules::system::{self, SystemInfo};
use sib_modules::{cpu, gpu};

use crate::audit::context::AuditContext;
use crate::audit::evidence::{EvidenceSource, TAB_GPU, TAB_PROCESSES, TAB_RESOURCES};
use crate::audit::pattern::{Area, Pattern, Verdict, Weight};

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
const NO_DATA: &str = "модуль не собрал данные";

const ADVICE_CPU: &str = "Найдите процессы-потребители на вкладке «Процессы». При постоянной нагрузке добавьте ядер или разнесите сервисы.";
const ADVICE_LOAD: &str = "Load выше числа ядер означает очередь на CPU или ожидание диска. Смотрите «Процессы» и I/O дисков.";
const ADVICE_MEMORY: &str = "Проверьте процессы и контейнеры с наибольшим потреблением, добавьте памяти или задайте лимиты контейнерам.";
const ADVICE_SWAP: &str =
    "Уменьшите потребление памяти или увеличьте RAM: обмен со swap замедляет работу.";
const ADVICE_OOM: &str =
    "Найдите процесс в журнале (dmesg, journalctl -k) и ограничьте его потребление памяти.";
const ADVICE_DISK: &str = "Освободите место: журналы (journalctl --vacuum-size), кэш пакетов, старые образы Docker (docker system prune).";
const ADVICE_INODES: &str = "Найдите каталог с большим числом мелких файлов (кэши, сессии, почта).";
const ADVICE_ZOMBIES: &str =
    "Перезапустите родительский процесс: он не читает статус завершившихся потомков.";
const ADVICE_GPU: &str = "Проверьте охлаждение и нагрузку на GPU; при перегреве снижается частота.";

pub static PATTERNS: &[Pattern] = &[
    Pattern {
        id: "resources.cpu_hour",
        area: Area::Resources,
        subject: "Загрузка CPU за час",
        description: "Среднее по точкам за последний час: постоянная высокая загрузка означает нехватку ядер или процесс в цикле.",
        weight: Weight::Medium,
        advice: ADVICE_CPU,
        evidence: EvidenceSource::Tab(TAB_PROCESSES),
        evaluate: cpu_hour,
    },
    Pattern {
        id: "resources.load",
        area: Area::Resources,
        subject: "Load average",
        description: "Отношение load5 к числу ядер: значение больше единицы означает ожидание CPU или диска.",
        weight: Weight::Medium,
        advice: ADVICE_LOAD,
        evidence: EvidenceSource::Tab(TAB_PROCESSES),
        evaluate: load,
    },
    Pattern {
        id: "resources.memory",
        area: Area::Resources,
        subject: "Память",
        description: "Доля занятой памяти без учёта кэша страниц.",
        weight: Weight::Medium,
        advice: ADVICE_MEMORY,
        evidence: EvidenceSource::Tab(TAB_RESOURCES),
        evaluate: memory,
    },
    Pattern {
        id: "resources.oom",
        area: Area::Resources,
        subject: "OOM killer",
        description: "Срабатывания OOM killer: при нехватке памяти ядро завершает процесс.",
        weight: Weight::Medium,
        advice: ADVICE_OOM,
        evidence: EvidenceSource::Tab(TAB_RESOURCES),
        evaluate: oom,
    },
    Pattern {
        id: "resources.swap",
        area: Area::Resources,
        subject: "Использование swap",
        description: "Занятый swap означает, что рабочий набор процессов не помещается в RAM.",
        weight: Weight::Low,
        advice: ADVICE_SWAP,
        evidence: EvidenceSource::Tab(TAB_RESOURCES),
        evaluate: swap,
    },
    Pattern {
        id: "resources.disk_space",
        area: Area::Resources,
        subject: "Место на",
        description: "Заполнение файловой системы: при 100 % прекращается запись журналов и баз данных.",
        weight: Weight::High,
        advice: ADVICE_DISK,
        evidence: EvidenceSource::Tab(TAB_RESOURCES),
        evaluate: disk_space,
    },
    Pattern {
        id: "resources.inodes",
        area: Area::Resources,
        subject: "Inode на",
        description: "При исчерпании inode запись прекращается, хотя свободное место на диске есть.",
        weight: Weight::Medium,
        advice: ADVICE_INODES,
        evidence: EvidenceSource::Tab(TAB_RESOURCES),
        evaluate: inodes,
    },
    Pattern {
        id: "resources.zombies",
        area: Area::Resources,
        subject: "Зомби-процессы",
        description: "Процессы, завершившиеся, но не прочитанные родителем. В большом количестве исчерпывают таблицу процессов.",
        weight: Weight::Low,
        advice: ADVICE_ZOMBIES,
        evidence: EvidenceSource::Tab(TAB_PROCESSES),
        evaluate: zombies,
    },
    Pattern {
        id: "resources.gpu_temperature",
        area: Area::Resources,
        subject: "Температура GPU",
        description: "Перегрев GPU включает троттлинг и сокращает срок службы.",
        weight: Weight::Low,
        advice: ADVICE_GPU,
        evidence: EvidenceSource::Tab(TAB_GPU),
        evaluate: gpu_temperature,
    },
];

fn cpu_hour(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(series) = ctx.server.series.get(cpu::KEY_TOTAL) else {
        return Verdict::skipped(NO_DATA).single();
    };
    let points: Vec<f64> = series.since(Duration::hours(1)).map(|p| p.value).collect();
    if points.is_empty() {
        return Verdict::skipped("нет точек за час").single();
    }
    let average = points.iter().sum::<f64>() / points.len() as f64;
    Verdict::graded(
        average >= CPU_FAIL_PCT,
        average >= CPU_WARN_PCT,
        format!("среднее {average:.0}% по {} точкам", points.len()),
    )
    .single()
}

fn load(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(info) = ctx.data::<SystemInfo>(system::ID) else {
        return Verdict::skipped(NO_DATA).single();
    };
    let cores = info.cpu_cores.max(1) as f64;
    let ratio = info.load.five / cores;
    Verdict::graded(
        ratio >= LOAD_FAIL_RATIO,
        ratio >= LOAD_WARN_RATIO,
        format!("load5 {:.2} на {} ядер", info.load.five, info.cpu_cores),
    )
    .single()
}

fn memory(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(snapshot) = ctx.data::<MemorySnapshot>(memory::ID) else {
        return Verdict::skipped(NO_DATA).single();
    };
    let used = snapshot.used_pct();
    Verdict::graded(
        used >= MEMORY_FAIL_PCT,
        used >= MEMORY_WARN_PCT,
        format!("занято {used:.0}%"),
    )
    .single()
}

fn oom(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(snapshot) = ctx.data::<MemorySnapshot>(memory::ID) else {
        return Verdict::skipped(NO_DATA).single();
    };
    let recent = snapshot.recent_oom_kills(chrono::Utc::now());
    Verdict::graded(
        false,
        recent > 0,
        format!("{recent} за сутки, {} с загрузки", snapshot.oom_kills),
    )
    .single()
}

fn swap(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(snapshot) = ctx.data::<MemorySnapshot>(memory::ID) else {
        return Verdict::skipped(NO_DATA).single();
    };
    if snapshot.swap_total_bytes == 0 {
        return Verdict::skipped("swap не настроен").single();
    }
    let used = snapshot.swap_used_pct();
    Verdict::graded(
        used >= SWAP_FAIL_PCT,
        used >= SWAP_WARN_PCT,
        format!("занято {used:.0}%"),
    )
    .single()
}

fn disk_space(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(snapshot) = ctx.data::<DiskSnapshot>(disk::ID) else {
        return Verdict::skipped(NO_DATA).single();
    };
    snapshot
        .filesystems
        .iter()
        .map(|fs| {
            let used = fs.used_pct();
            Verdict::graded(
                used >= DISK_FAIL_PCT,
                used >= DISK_WARN_PCT,
                format!(
                    "занято {used:.0}%, свободно {} MiB",
                    fs.available_bytes / 1024 / 1024
                ),
            )
            .for_instance(&fs.mount)
        })
        .collect()
}

fn inodes(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(snapshot) = ctx.data::<DiskSnapshot>(disk::ID) else {
        return Verdict::skipped(NO_DATA).single();
    };
    snapshot
        .filesystems
        .iter()
        .filter(|fs| fs.inodes_total > 0)
        .map(|fs| {
            let used = fs.inodes_used_pct();
            Verdict::graded(
                used >= INODES_FAIL_PCT,
                used >= INODES_WARN_PCT,
                format!("занято {used:.0}%"),
            )
            .for_instance(&fs.mount)
        })
        .collect()
}

fn zombies(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(snapshot) = ctx.data::<ProcessSnapshot>(processes::ID) else {
        return Verdict::skipped(NO_DATA).single();
    };
    let zombies = snapshot.zombie_count();
    Verdict::graded(
        zombies >= ZOMBIES_FAIL,
        zombies >= ZOMBIES_WARN,
        format!("{zombies} зомби из {} процессов", snapshot.processes.len()),
    )
    .single()
}

fn gpu_temperature(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(snapshot) = ctx.data::<gpu::GpuSnapshot>(gpu::ID) else {
        return Vec::new();
    };
    snapshot
        .gpus
        .iter()
        .filter_map(|card| {
            let temperature = card.temperature_c?;
            Some(
                Verdict::graded(
                    temperature >= GPU_TEMP_FAIL,
                    temperature >= GPU_TEMP_WARN,
                    format!("{temperature:.0} °C, память {:.0}%", card.memory_pct()),
                )
                .for_instance(card.index.to_string()),
            )
        })
        .collect()
}
