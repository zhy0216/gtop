use gpui_component::IconName;
use sysinfo::Pid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MonitorTab {
    #[default]
    System = 0,
    Processes = 1,
}

impl MonitorTab {
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => MonitorTab::System,
            1 => MonitorTab::Processes,
            _ => MonitorTab::System,
        }
    }
}

#[derive(Clone)]
pub struct MetricPoint {
    pub time: String,
    pub cpu: f64,
    pub memory: f64,
}

#[derive(Clone)]
pub struct CpuCoreUsage {
    pub usage: f32,
}

#[derive(Clone)]
pub struct NetworkStats {
    pub rx_bytes_per_sec: u64,
    pub tx_bytes_per_sec: u64,
}

#[derive(Clone)]
pub struct ProcessInfo {
    pub pid: Pid,
    pub name: String,
    pub cpu_usage: f32,
    pub memory: u64,
}

#[derive(Clone)]
pub struct DiskInfo {
    pub total: u64,
    pub used: u64,
}

#[derive(Clone)]
pub struct BatteryInfo {
    pub icon: IconName,
    pub percentage: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProcessSortField {
    Pid,
    Name,
    #[default]
    Cpu,
    Memory,
}

pub struct SmoothedMetrics {
    pub cpu: f64,
    pub memory: f64,
    pub swap_percent: f64,
    pub disk_used: u64,
    pub disk_total: u64,
}
