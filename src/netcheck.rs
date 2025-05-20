use std::collections::HashSet;
use std::net::IpAddr;
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use tokio::time::sleep;

pub struct NetworkMonitor {
    last_check_time: Instant,
    last_ip_addresses: HashSet<String>,
}

impl NetworkMonitor {
    pub fn new() -> Self {
        Self {
            last_check_time: Instant::now(),
            last_ip_addresses: HashSet::new(),
        }
    }

    pub async fn is_network_changed(&mut self) -> bool {
        // Don't check too frequently
        if self.last_check_time.elapsed() < Duration::from_secs(5) {
            return false;
        }

        if let Ok(current_ips) = self.get_current_ip_addresses() {
            let current_set: HashSet<String> = current_ips.into_iter().collect();

            // Check if IP addresses have changed
            if self.last_ip_addresses.is_empty() {
                // First run, initialize the set
                self.last_ip_addresses = current_set;
                self.last_check_time = Instant::now();
                return false;
            } else if current_set != self.last_ip_addresses {
                // IP addresses have changed
                println!("Network interfaces changed");
                self.last_ip_addresses = current_set;
                self.last_check_time = Instant::now();
                return true;
            }

            // Check if we can reach the forward servers
            if !self.can_reach_servers().await {
                println!("Cannot reach forward servers");
                self.last_check_time = Instant::now();
                return true;
            }
        }

        self.last_check_time = Instant::now();
        false
    }

    fn get_current_ip_addresses(&self) -> Result<Vec<String>> {
        let output = if cfg!(target_os = "windows") {
            Command::new("ipconfig").output()?
        } else {
            Command::new("ip").arg("addr").output()?
        };

        if !output.status.success() {
            return Err(anyhow!("Failed to get network interfaces"));
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        let mut ip_addresses = Vec::new();

        // Simple regex-like parsing for IP addresses (this is very basic)
        for line in output_str.lines() {
            if line.contains("IPv4") || line.contains("inet ") {
                for word in line.split_whitespace() {
                    if word.contains('.') {
                        // Very basic IPv4 validation
                        if let Ok(ip) = word.parse::<IpAddr>() {
                            if ip.is_ipv4() && !ip.is_loopback() {
                                ip_addresses.push(ip.to_string());
                            }
                        }
                    }
                }
            }
        }

        Ok(ip_addresses)
    }

    pub async fn can_reach_servers(&self) -> bool {
        // Try to ping both servers
        let servers = vec!["rustdesk.ntsports.tech", "jpm.holomotion.tech"];

        for server in servers {
            if self.can_ping(server).await {
                return true; // If we can reach at least one server, network is considered up
            }
        }

        false
    }

    async fn can_ping(&self, host: &str) -> bool {
        let ping_command = if cfg!(target_os = "windows") {
            Command::new("ping")
                .args(["-n", "1", "-w", "1000", host])
                .output()
        } else {
            Command::new("ping")
                .args(["-c", "1", "-W", "1", host])
                .output()
        };

        match ping_command {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }

    pub async fn wait_for_network(&self) -> bool {
        for _ in 0..30 {  // Wait up to 30 seconds
            if self.can_reach_servers().await {
                return true;
            }
            sleep(Duration::from_secs(1)).await;
        }
        false
    }
}