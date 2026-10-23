use serde::Serialize;
use std::fs;
use std::sync::Mutex;

#[derive(Serialize, Debug, PartialEq)]
pub struct Load {
    pub cpu: f64,
    pub used: u64,
    pub total: u64,
}

#[derive(Default)]
pub struct Cpu(Mutex<Option<(u64, u64)>>);

pub fn ticks(stat: &str) -> Option<(u64, u64)> {
    let fields: Vec<u64> = stat
        .lines()
        .next()?
        .strip_prefix("cpu ")?
        .split_whitespace()
        .filter_map(|n| n.parse().ok())
        .collect();
    let idle = fields.get(3)? + fields.get(4).unwrap_or(&0);
    Some((idle, fields.iter().take(8).sum()))
}

pub fn memory(meminfo: &str) -> Option<(u64, u64)> {
    let kib = |key: &str| -> Option<u64> {
        meminfo
            .lines()
            .find_map(|l| l.strip_prefix(key))?
            .split_whitespace()
            .next()?
            .parse::<u64>()
            .ok()
            .map(|k| k * 1024)
    };
    let total = kib("MemTotal:")?;
    Some((total - kib("MemAvailable:")?, total))
}

impl Cpu {
    pub fn load(&self) -> Option<Load> {
        let (idle, all) = ticks(&fs::read_to_string("/proc/stat").ok()?)?;
        let (used, total) = memory(&fs::read_to_string("/proc/meminfo").ok()?)?;
        let previous = self.0.lock().unwrap().replace((idle, all));
        let cpu = previous
            .filter(|(_, before)| all > *before)
            .map(|(idle0, all0)| 100.0 * (1.0 - (idle - idle0) as f64 / (all - all0) as f64))
            .unwrap_or(0.0);
        Some(Load { cpu, used, total })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_cpu_ticks_and_memory_from_proc() {
        let stat = "cpu  100 5 50 800 20 0 5 0 0 0\ncpu0 1 2 3 4\n";
        assert_eq!(ticks(stat), Some((820, 980)));
        let meminfo =
            "MemTotal:       32000000 kB\nMemFree:  1000 kB\nMemAvailable:   12000000 kB\n";
        assert_eq!(memory(meminfo), Some((20000000 * 1024, 32000000 * 1024)));
        assert_eq!(ticks("intr 1 2"), None);
    }

    #[test]
    fn measures_the_real_machine() {
        let cpu = Cpu::default();
        cpu.load().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        let load = cpu.load().unwrap();
        assert!((0.0..=100.0).contains(&load.cpu));
        assert!(load.used > 0 && load.used < load.total);
    }
}
