use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use russh::client::Handle;
use tokio::sync::RwLock;

use super::{history::MetricHistory, parser, HostMetrics};

pub struct HostMetricsCollector {
    metrics: Arc<RwLock<HostMetrics>>,
    pub cpu_history: Arc<RwLock<MetricHistory>>,
    pub mem_history: Arc<RwLock<MetricHistory>>,
    pub net_rx_history: Arc<RwLock<MetricHistory>>,
    pub net_tx_history: Arc<RwLock<MetricHistory>>,
    prev_cpu_raw: Arc<RwLock<String>>,
    prev_net_raw: Arc<RwLock<String>>,
    last_net_time: Arc<RwLock<Instant>>,
    process_count: usize,
}

impl HostMetricsCollector {
    pub fn new(history_samples: usize, process_count: usize) -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HostMetrics::default())),
            cpu_history: Arc::new(RwLock::new(MetricHistory::new(history_samples))),
            mem_history: Arc::new(RwLock::new(MetricHistory::new(history_samples))),
            net_rx_history: Arc::new(RwLock::new(MetricHistory::new(history_samples))),
            net_tx_history: Arc::new(RwLock::new(MetricHistory::new(history_samples))),
            prev_cpu_raw: Arc::new(RwLock::new(String::new())),
            prev_net_raw: Arc::new(RwLock::new(String::new())),
            last_net_time: Arc::new(RwLock::new(Instant::now())),
            process_count,
        }
    }

    pub fn metrics(&self) -> Arc<RwLock<HostMetrics>> {
        Arc::clone(&self.metrics)
    }

    pub fn cpu_history(&self) -> Arc<RwLock<MetricHistory>> {
        Arc::clone(&self.cpu_history)
    }

    pub fn mem_history(&self) -> Arc<RwLock<MetricHistory>> {
        Arc::clone(&self.mem_history)
    }

    pub fn net_rx_history(&self) -> Arc<RwLock<MetricHistory>> {
        Arc::clone(&self.net_rx_history)
    }

    pub fn net_tx_history(&self) -> Arc<RwLock<MetricHistory>> {
        Arc::clone(&self.net_tx_history)
    }

    /// Execute a single command on the remote host and return its stdout.
    async fn exec_remote<H: russh::client::Handler>(
        handle: &Handle<H>,
        command: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let mut channel = handle.channel_open_session().await?;
        channel.exec(true, command.as_bytes()).await?;
        let mut output = Vec::new();
        while let Some(msg) = channel.wait().await {
            match msg {
                russh::ChannelMsg::Data { data } => output.extend_from_slice(&data),
                russh::ChannelMsg::Eof | russh::ChannelMsg::Close => break,
                russh::ChannelMsg::ExitStatus { .. } => break,
                _ => {}
            }
        }
        Ok(String::from_utf8_lossy(&output).to_string())
    }

    /// Collect all metrics from the remote host in a single batch.
    /// Runs a combined command to minimize round trips.
    pub async fn collect<H: russh::client::Handler>(
        &self,
        handle: &Handle<H>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let combined = concat!(
            "echo '===CPUSTAT==='; cat /proc/stat; ",
            "echo '===MEMINFO==='; cat /proc/meminfo; ",
            "echo '===LOADAVG==='; cat /proc/loadavg; ",
            "echo '===DF==='; df -P; ",
            "echo '===NETDEV==='; cat /proc/net/dev; ",
            "echo '===UPTIME==='; cat /proc/uptime; ",
            "echo '===PS==='; ps aux --sort=-%cpu 2>/dev/null || ps aux; ",
            "echo '===END==='"
        );

        let raw = Self::exec_remote(handle, combined).await?;

        let sections = split_sections(&raw);

        let cpu_raw = sections.get("CPUSTAT").cloned().unwrap_or_default();
        let meminfo_raw = sections.get("MEMINFO").cloned().unwrap_or_default();
        let loadavg_raw = sections.get("LOADAVG").cloned().unwrap_or_default();
        let df_raw = sections.get("DF").cloned().unwrap_or_default();
        let net_raw = sections.get("NETDEV").cloned().unwrap_or_default();
        let uptime_raw = sections.get("UPTIME").cloned().unwrap_or_default();
        let ps_raw = sections.get("PS").cloned().unwrap_or_default();

        // Parse CPU with delta
        let prev_cpu = self.prev_cpu_raw.read().await.clone();
        let (cpu_pct, cpu_per_core) = if prev_cpu.is_empty() {
            (0.0, Vec::new())
        } else {
            parser::parse_cpu(&cpu_raw, &prev_cpu)
        };
        *self.prev_cpu_raw.write().await = cpu_raw;

        // Parse memory
        let (mem_total, mem_used, mem_avail, swap_total, swap_used) =
            parser::parse_meminfo(&meminfo_raw);

        // Parse load
        let (l1, l5, l15) = parser::parse_loadavg(&loadavg_raw);

        // Parse disks
        let disks = parser::parse_df(&df_raw);

        // Parse network with delta
        let prev_net = self.prev_net_raw.read().await.clone();
        let now = Instant::now();
        let last_time = *self.last_net_time.read().await;
        let elapsed = now.duration_since(last_time).as_secs_f64();
        let (net_rx, net_tx) = if prev_net.is_empty() {
            (0.0, 0.0)
        } else {
            parser::parse_net_dev(&net_raw, &prev_net, elapsed)
        };
        *self.prev_net_raw.write().await = net_raw;
        *self.last_net_time.write().await = now;

        // Parse uptime
        let uptime = parser::parse_uptime(&uptime_raw);

        // Parse processes
        let top_cpu = parser::parse_ps(&ps_raw, self.process_count);
        let mut top_mem = top_cpu.clone();
        top_mem.sort_by(|a, b| {
            b.mem_pct
                .partial_cmp(&a.mem_pct)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        top_mem.truncate(self.process_count);

        // Update metrics
        {
            let mut m = self.metrics.write().await;
            m.cpu_percent = cpu_pct;
            m.cpu_per_core = cpu_per_core;
            m.mem_total_kb = mem_total;
            m.mem_used_kb = mem_used;
            m.mem_available_kb = mem_avail;
            m.mem_swap_total_kb = swap_total;
            m.mem_swap_used_kb = swap_used;
            m.load_1m = l1;
            m.load_5m = l5;
            m.load_15m = l15;
            m.disks = disks;
            m.net_rx_bps = net_rx;
            m.net_tx_bps = net_tx;
            m.top_procs_cpu = top_cpu;
            m.top_procs_mem = top_mem;
            m.uptime_secs = uptime;
        }

        // Update histories
        self.cpu_history.write().await.push(cpu_pct as u64);
        let mem_pct = if mem_total > 0 {
            (mem_used as f64 / mem_total as f64 * 100.0) as u64
        } else {
            0
        };
        self.mem_history.write().await.push(mem_pct);
        self.net_rx_history
            .write()
            .await
            .push((net_rx / 1024.0) as u64);
        self.net_tx_history
            .write()
            .await
            .push((net_tx / 1024.0) as u64);

        Ok(())
    }
}

fn split_sections(raw: &str) -> HashMap<String, String> {
    let mut sections = HashMap::new();
    let mut current_key = String::new();
    let mut current_content = String::new();

    for line in raw.lines() {
        if line.starts_with("===") && line.ends_with("===") {
            if !current_key.is_empty() {
                sections.insert(current_key.clone(), current_content.trim().to_string());
            }
            current_key = line.trim_matches('=').to_string();
            current_content = String::new();
        } else if current_key != "END" {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }
    if !current_key.is_empty() && current_key != "END" {
        sections.insert(current_key, current_content.trim().to_string());
    }
    sections
}
