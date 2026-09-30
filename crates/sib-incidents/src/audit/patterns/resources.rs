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
const NO_DATA: &str = "the module has not collected data";

const ADVICE_CPU: &str = "Find the heavy processes on the Processes tab. If the load never drops, add cores or move services apart.";
const ADVICE_LOAD: &str = "A load above the core count means a queue for the CPU or waiting on disk. Look at Processes and at disk I/O.";
const ADVICE_MEMORY: &str = "Check the processes and containers that use the most memory, add RAM or set limits on the containers.";
const ADVICE_SWAP: &str = "Use less memory or add RAM. Swapping slows everything down.";
const ADVICE_OOM: &str =
    "Find the process in the journal (dmesg, journalctl -k) and limit how much memory it can take.";
const ADVICE_DISK: &str = "Free up space: journals (journalctl --vacuum-size), the package cache, old Docker images (docker system prune).";
const ADVICE_INODES: &str = "Find the directory with many small files (caches, sessions, mail).";
const ADVICE_ZOMBIES: &str =
    "Restart the parent process. It is not reading the status of its finished children.";
const ADVICE_GPU: &str =
    "Check the cooling and the load on the GPU. When it overheats the clock drops.";

pub static PATTERNS: &[Pattern] = &[
    Pattern {
        id: "resources.cpu_hour",
        area: Area::Resources,
        subject: "CPU load over an hour",
        description: "The average over the points of the last hour. A load that stays high means too few cores or a process stuck in a loop.",
        weight: Weight::Medium,
        advice: ADVICE_CPU,
        evidence: EvidenceSource::Tab(TAB_PROCESSES),
        evaluate: cpu_hour,
    },
    Pattern {
        id: "resources.load",
        area: Area::Resources,
        subject: "Load average",
        description: "load5 divided by the number of cores. Above one means waiting for the CPU or for disk.",
        weight: Weight::Medium,
        advice: ADVICE_LOAD,
        evidence: EvidenceSource::Tab(TAB_PROCESSES),
        evaluate: load,
    },
    Pattern {
        id: "resources.memory",
        area: Area::Resources,
        subject: "Memory",
        description: "How much memory is used, not counting the page cache.",
        weight: Weight::Medium,
        advice: ADVICE_MEMORY,
        evidence: EvidenceSource::Tab(TAB_RESOURCES),
        evaluate: memory,
    },
    Pattern {
        id: "resources.oom",
        area: Area::Resources,
        subject: "OOM killer",
        description: "How often the OOM killer fired. When memory runs out the kernel stops a process.",
        weight: Weight::Medium,
        advice: ADVICE_OOM,
        evidence: EvidenceSource::Tab(TAB_RESOURCES),
        evaluate: oom,
    },
    Pattern {
        id: "resources.swap",
        area: Area::Resources,
        subject: "Swap usage",
        description: "Used swap means the working set of the processes does not fit in RAM.",
        weight: Weight::Low,
        advice: ADVICE_SWAP,
        evidence: EvidenceSource::Tab(TAB_RESOURCES),
        evaluate: swap,
    },
    Pattern {
        id: "resources.disk_space",
        area: Area::Resources,
        subject: "Space on",
        description: "How full the file system is. At 100 % journals and databases stop writing.",
        weight: Weight::High,
        advice: ADVICE_DISK,
        evidence: EvidenceSource::Tab(TAB_RESOURCES),
        evaluate: disk_space,
    },
    Pattern {
        id: "resources.inodes",
        area: Area::Resources,
        subject: "Inodes on",
        description: "When inodes run out writing stops, even though there is free space on the disk.",
        weight: Weight::Medium,
        advice: ADVICE_INODES,
        evidence: EvidenceSource::Tab(TAB_RESOURCES),
        evaluate: inodes,
    },
    Pattern {
        id: "resources.zombies",
        area: Area::Resources,
        subject: "Zombie processes",
        description: "Processes that finished but whose status the parent never read. In large numbers they fill the process table.",
        weight: Weight::Low,
        advice: ADVICE_ZOMBIES,
        evidence: EvidenceSource::Tab(TAB_PROCESSES),
        evaluate: zombies,
    },
    Pattern {
        id: "resources.gpu_temperature",
        area: Area::Resources,
        subject: "GPU temperature",
        description: "An overheating GPU throttles itself and wears out sooner.",
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
        return Verdict::skipped("no points for the last hour").single();
    }
    let average = points.iter().sum::<f64>() / points.len() as f64;
    Verdict::graded(
        average >= CPU_FAIL_PCT,
        average >= CPU_WARN_PCT,
        format!("{average:.0}% on average over {} points", points.len()),
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
        format!("load5 {:.2} on {} cores", info.load.five, info.cpu_cores),
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
        format!("{used:.0}% used"),
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
        format!("{recent} in a day, {} since boot", snapshot.oom_kills),
    )
    .single()
}

fn swap(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(snapshot) = ctx.data::<MemorySnapshot>(memory::ID) else {
        return Verdict::skipped(NO_DATA).single();
    };
    if snapshot.swap_total_bytes == 0 {
        return Verdict::skipped("no swap set up").single();
    }
    let used = snapshot.swap_used_pct();
    Verdict::graded(
        used >= SWAP_FAIL_PCT,
        used >= SWAP_WARN_PCT,
        format!("{used:.0}% used"),
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
                    "{used:.0}% used, {} MiB free",
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
                format!("{used:.0}% used"),
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
        format!(
            "{zombies} zombies out of {} processes",
            snapshot.processes.len()
        ),
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
                    format!("{temperature:.0} C, memory {:.0}%", card.memory_pct()),
                )
                .for_instance(card.index.to_string()),
            )
        })
        .collect()
}
