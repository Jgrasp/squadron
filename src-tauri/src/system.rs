use serde::Serialize;
use std::fs;
use std::sync::Mutex;

#[derive(Serialize, Debug, PartialEq)]
pub struct Load {
    pub cpu: f64,
    pub used: u64,
    pub total: u64,
}

#[cfg(not(target_os = "macos"))]
#[derive(Default)]
pub struct Cpu(Mutex<Option<(u64, u64)>>);

#[cfg(target_os = "macos")]
pub struct Cpu(Mutex<sysinfo::System>);

#[cfg(target_os = "macos")]
impl Default for Cpu {
    fn default() -> Self {
        Self(Mutex::new(sysinfo::System::new()))
    }
}

#[cfg(target_os = "macos")]
impl Cpu {
    pub fn load(&self) -> Option<Load> {
        let mut system = self.0.lock().unwrap();
        system.refresh_cpu_usage();
        system.refresh_memory();
        Some(Load {
            cpu: system.global_cpu_usage() as f64,
            used: system.used_memory(),
            total: system.total_memory(),
        })
    }
}

#[cfg_attr(target_os = "macos", allow(dead_code))]
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

#[cfg_attr(target_os = "macos", allow(dead_code))]
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

#[cfg(not(target_os = "macos"))]
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

#[cfg(target_os = "linux")]
pub fn package_manager_busy() -> bool {
    use std::os::unix::fs::MetadataExt;
    if std::env::var_os("APPIMAGE").is_some() {
        return false;
    }
    let Ok(locks) = fs::read_to_string("/proc/locks") else {
        return false;
    };
    ["/var/lib/dpkg/lock-frontend", "/var/lib/dpkg/lock"]
        .iter()
        .filter_map(|path| fs::metadata(path).ok())
        .any(|m| locked(&locks, m.dev(), m.ino()))
}

#[cfg(not(target_os = "linux"))]
pub fn package_manager_busy() -> bool {
    false
}

#[cfg(target_os = "linux")]
fn locked(locks: &str, dev: u64, ino: u64) -> bool {
    let id = format!("{:02x}:{:02x}:{ino}", libc::major(dev as _), libc::minor(dev as _));
    locks
        .lines()
        .filter_map(|l| l.split_whitespace().nth(5))
        .any(|file| file == id)
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

    #[cfg(target_os = "linux")]
    #[test]
    fn finds_the_file_in_the_kernel_locks() {
        let dev = libc::makedev(259, 1);
        let locks = "1: POSIX  ADVISORY  WRITE 14024 103:01:4207979 0 EOF\n2: FLOCK  ADVISORY  WRITE 2793 103:03:6059362 0 EOF\n";
        assert!(locked(locks, dev, 4207979));
        assert!(!locked(locks, dev, 6059362));
        assert!(!locked(locks, libc::makedev(259, 3), 4207979));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn sees_a_lock_held_on_the_machine() {
        use std::os::unix::fs::MetadataExt;
        use std::os::unix::io::AsRawFd;
        let path = std::env::temp_dir().join(format!("squadron-lock-{}", std::process::id()));
        let file = fs::OpenOptions::new().create(true).write(true).open(&path).unwrap();
        let meta = file.metadata().unwrap();
        let read = || fs::read_to_string("/proc/locks").unwrap();
        assert!(!locked(&read(), meta.dev(), meta.ino()));
        let mut lock: libc::flock = unsafe { std::mem::zeroed() };
        lock.l_type = libc::F_WRLCK as _;
        assert_eq!(unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETLK, &lock) }, 0);
        assert!(locked(&read(), meta.dev(), meta.ino()));
        fs::remove_file(path).unwrap();
    }
}
