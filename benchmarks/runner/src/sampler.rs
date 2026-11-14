//! Memory and CPU sampling of an app's whole process tree.
//!
//! Memory is the sum over the tree of each process's proportional set size (PSS) on Linux,
//! which splits shared pages between the processes that map them, and of the resident set
//! size (RSS) elsewhere. CPU time is the sum of user and system time over the tree.

use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    time::Duration,
};

use serde::{Deserialize, Serialize};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

/// The memory metric this platform reports.
pub fn memory_metric() -> &'static str {
    if cfg!(target_os = "linux") {
        "pss"
    } else {
        "rss"
    }
}

/// One snapshot of a process tree.
struct Snapshot {
    memory: u64,
    /// Accumulated CPU milliseconds per process.
    cpu: HashMap<Pid, u64>,
}

struct Tree {
    system: System,
    root: Pid,
}

impl Tree {
    fn new(root: u32) -> Option<Self> {
        Some(Self {
            system: System::new().ok()?,
            root: Pid::from_u32(root),
        })
    }

    fn snapshot(&mut self) -> Snapshot {
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_memory().with_cpu(),
        );
        let processes = self.system.processes();
        let mut members = HashSet::from([self.root]);
        // Parents can appear after their children in the map, so repeat until stable.
        loop {
            let before = members.len();
            for (pid, process) in processes {
                if let Some(parent) = process.parent()
                    && members.contains(&parent)
                {
                    members.insert(*pid);
                }
            }
            if members.len() == before {
                break;
            }
        }
        let mut snapshot = Snapshot {
            memory: 0,
            cpu: HashMap::new(),
        };
        for pid in members {
            if let Some(process) = processes.get(&pid) {
                snapshot.memory += pss(pid).unwrap_or_else(|| process.memory());
                snapshot.cpu.insert(pid, process.accumulated_cpu_time());
            }
        }
        snapshot
    }
}

#[cfg(target_os = "linux")]
fn pss(pid: Pid) -> Option<u64> {
    let text = std::fs::read_to_string(format!("/proc/{pid}/smaps_rollup")).ok()?;
    let line = text.lines().find(|line| line.starts_with("Pss:"))?;
    let kib: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kib * 1024)
}

#[cfg(not(target_os = "linux"))]
fn pss(_pid: Pid) -> Option<u64> {
    None
}

/// Memory of the process tree rooted at `root` right now.
pub fn memory_now(root: u32) -> Option<u64> {
    let mut tree = Tree::new(root)?;
    Some(tree.snapshot().memory)
}

/// Resource usage of an app over a measurement window.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub peak_memory_bytes: u64,
    pub mean_memory_bytes: u64,
    pub cpu_seconds: f64,
    pub samples: u32,
}

/// Samples a process tree on a background thread until stopped.
pub struct Sampler {
    stop: Arc<AtomicBool>,
    handle: JoinHandle<Usage>,
}

impl Sampler {
    pub fn start(root: u32, interval: Duration) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let handle = std::thread::spawn(move || sample(root, interval, &flag));
        Self { stop, handle }
    }

    pub fn finish(self) -> Usage {
        self.stop.store(true, Ordering::SeqCst);
        self.handle.join().unwrap_or_default()
    }
}

fn sample(root: u32, interval: Duration, stop: &AtomicBool) -> Usage {
    let Some(mut tree) = Tree::new(root) else {
        return Usage::default();
    };
    let first = tree.snapshot();
    let start_cpu = first.cpu.clone();
    // The latest CPU time seen per process, so workers that exit mid-window still count.
    let mut last_cpu = first.cpu;
    let mut peak = first.memory;
    let mut total: u128 = u128::from(first.memory);
    let mut samples = 1u32;
    loop {
        // Sleep in small steps so `finish` returns promptly.
        let deadline = std::time::Instant::now() + interval;
        while !stop.load(Ordering::SeqCst) {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                break;
            }
            std::thread::sleep(remaining.min(Duration::from_millis(20)));
        }
        let stopping = stop.load(Ordering::SeqCst);
        let snapshot = tree.snapshot();
        peak = peak.max(snapshot.memory);
        total += u128::from(snapshot.memory);
        samples += 1;
        last_cpu.extend(snapshot.cpu);
        if stopping {
            break;
        }
    }
    let cpu_millis: u64 = last_cpu
        .iter()
        .map(|(pid, cpu)| cpu.saturating_sub(start_cpu.get(pid).copied().unwrap_or(0)))
        .sum();
    #[expect(clippy::cast_precision_loss, reason = "milliseconds fit in f64")]
    let cpu_seconds = cpu_millis as f64 / 1000.0;
    Usage {
        peak_memory_bytes: peak,
        mean_memory_bytes: u64::try_from(total / u128::from(samples)).unwrap_or(u64::MAX),
        cpu_seconds,
        samples,
    }
}
