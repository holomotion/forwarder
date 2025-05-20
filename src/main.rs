use std::sync::Arc;
use std::time::Duration;

use crate::country::get_current_country;
use crate::reporter::{ForwardEntry, ForwardInfo};
use anyhow::Result;
use bore_cli::client;
use mac_address::{get_mac_address, MacAddressIterator};
use rand::Rng;
use self_github_update_enhanced::{backends::github, cargo_crate_version};
use tokio::sync::Mutex;
use tokio::time::{interval, sleep, timeout};
use netcheck::NetworkMonitor;

mod reporter;
mod hostname;
mod access_id;
mod country;
mod netcheck;

const LOCALHOST: &str = "localhost";
const FORWARD_SERVER: &str = "rustdesk.ntsports.tech";
const FORWARD_JP_SERVER: &str = "jpm.holomotion.tech";
const FORWARD_SECRET: &str = "hm#CD888";
const NIL_MAC_ADDRESS: &str = "00:00:00:00:00:00";
// Backoff retry strategy parameters
const INITIAL_RETRY_INTERVAL: u64 = 2; // seconds
const MAX_RETRY_INTERVAL: u64 = 300; // 5 minutes max interval

struct ForwarderState {
    ssh_client: Option<client::Client>,
    cockpit_client: Option<client::Client>,
    forward_info: ForwardInfo,
    server_address: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let mac_address_result = get_mac_address()?;
    if let Some(mac_address) = mac_address_result {
        // apply the default settings
        let mut auto_selected_country = FORWARD_SERVER;
        let mut current_country_code = "CN".to_string();

        // auto select country endpoint
        let current_country_result = get_current_country().await;
        if let Ok(current_country) = current_country_result {
            current_country_code = current_country.country.clone();
            if current_country.country != "CN" {
                auto_selected_country = FORWARD_JP_SERVER;
            }
        }

        let all_mac_address_iter = MacAddressIterator::new()?;
        let all_mac_addresses = all_mac_address_iter.filter(|ma| ma.to_string() != NIL_MAC_ADDRESS).map(|ma| ma.to_string()).collect();

        // Create a shared state for forwarder
        let state = Arc::new(Mutex::new(ForwarderState {
            ssh_client: None,
            cockpit_client: None,
            forward_info: ForwardInfo {
                client_country: current_country_code,
                app_version: cargo_crate_version!().parse()?,
                hostname: hostname::get_hostname()?,
                access_id: access_id::get_access_id()?,
                mac_address: mac_address.to_string(),
                forward_entries: Vec::new(),
                all_mac_addresses,
            },
            server_address: auto_selected_country.to_string(),
        }));

        // Initialize connections
        if !initialize_connections(Arc::clone(&state)).await? {
            eprintln!("Failed to initialize connections");
            return Ok(());
        }

        // Network status tracking
        let network_status = Arc::new(Mutex::new(true)); // Assume network is initially available
        let network_status_for_monitor = Arc::clone(&network_status);
        let network_status_for_main = Arc::clone(&network_status);

        // Start network monitoring
        let state_for_network = Arc::clone(&state);

        let network_monitor_task = tokio::spawn(async move {
            let mut network_monitor = NetworkMonitor::new();
            let mut check_interval = interval(Duration::from_secs(10));
            let mut was_network_down = false;

            loop {
                check_interval.tick().await;

                // Check if network changed or went down/up
                if network_monitor.is_network_changed().await {
                    println!("Network status changed, checking connectivity...");

                    // Wait a moment for network to stabilize
                    sleep(Duration::from_secs(3)).await;

                    let is_network_up = network_monitor.can_reach_servers().await;
                    let mut network_status_guard = network_status_for_monitor.lock().await;

                    if !*network_status_guard && is_network_up {
                        // Network was down and is now up
                        println!("Network restored, attempting reconnection");
                        *network_status_guard = true;
                        was_network_down = true;
                    } else if *network_status_guard && !is_network_up {
                        // Network was up and is now down
                        println!("Network connectivity lost");
                        *network_status_guard = false;
                        was_network_down = true;
                    }

                    drop(network_status_guard);

                    if was_network_down && is_network_up {
                        was_network_down = false;

                        // Re-check country on network change
                        if let Ok(current_country) = get_current_country().await {
                            let mut state = state_for_network.lock().await;
                            state.forward_info.client_country = current_country.country.clone();
                            if current_country.country != "CN" {
                                state.server_address = FORWARD_JP_SERVER.to_string();
                            } else {
                                state.server_address = FORWARD_SERVER.to_string();
                            }
                        }

                        let _ = reconnect_clients(Arc::clone(&state_for_network)).await;
                    }
                }
            }
        });

        let check_update_task = tokio::spawn(
            timeout(Duration::from_secs(300), check_update())
        );

        let _ = tokio::join!(
            network_monitor_task,
            check_update_task,
        );
    }
    Ok(())
}

async fn initialize_connections(state: Arc<Mutex<ForwarderState>>) -> Result<bool> {
    let mut state_guard = state.lock().await;
    let server_address = state_guard.server_address.clone();

    // Initialize SSH client
    match client::Client::new(LOCALHOST, 22, &server_address, 0, Some(FORWARD_SECRET)).await {
        Ok(ssh_cli) => {
            let ssh_port = ssh_cli.remote_port();
            state_guard.ssh_client = Some(ssh_cli);

            // Initialize Cockpit client
            match client::Client::new(LOCALHOST, 9090, &server_address, 0, Some(FORWARD_SECRET)).await {
                Ok(cockpit_cli) => {
                    let cockpit_port = cockpit_cli.remote_port();
                    state_guard.cockpit_client = Some(cockpit_cli);

                    // Update forward entries
                    state_guard.forward_info.forward_entries = vec![
                        ForwardEntry {
                            local_host: LOCALHOST.parse()?,
                            local_port: 22,
                            remote_port: ssh_port,
                        },
                        ForwardEntry {
                            local_host: LOCALHOST.parse()?,
                            local_port: 9090,
                            remote_port: cockpit_port,
                        },
                    ];

                    // Report the new connection info
                    state_guard.forward_info.report().await?;

                    // Start listening tasks
                    let ssh_client = state_guard.ssh_client.take().unwrap();
                    let cockpit_client = state_guard.cockpit_client.take().unwrap();

                    drop(state_guard); // Release the lock before spawning tasks

                    tokio::spawn(async move {
                        if let Err(e) = ssh_client.listen().await {
                            eprintln!("SSH forward error: {:?}", e);
                        }
                    });

                    tokio::spawn(async move {
                        if let Err(e) = cockpit_client.listen().await {
                            eprintln!("Cockpit forward error: {:?}", e);
                        }
                    });

                    return Ok(true);
                },
                Err(e) => {
                    eprintln!("Failed to create cockpit client: {:?}", e);
                    return Ok(false);
                }
            }
        },
        Err(e) => {
            eprintln!("Failed to create SSH client: {:?}", e);
            return Ok(false);
        }
    }
}

async fn reconnect_clients(state: Arc<Mutex<ForwarderState>>) -> Result<bool> {
    let mut retry_interval = INITIAL_RETRY_INTERVAL;
    let mut consecutive_failures = 0;

    // Keep trying to reconnect indefinitely - never give up
    loop {
        println!("Reconnection attempt with interval: {}s", retry_interval);

        if initialize_connections(Arc::clone(&state)).await? {
            println!("Reconnection successful");
            return Ok(true);
        }

        // Update retry interval with exponential backoff
        consecutive_failures += 1;
        retry_interval = calculate_backoff(consecutive_failures);

        println!("Reconnection failed. Waiting {}s before next attempt", retry_interval);
        sleep(Duration::from_secs(retry_interval)).await;
    }
}

fn calculate_backoff(failures: u32) -> u64 {
    // Calculate exponential backoff with jitter: min(max_interval, initial * 2^n) ± jitter
    let base_interval = INITIAL_RETRY_INTERVAL * 2u64.saturating_pow(failures);
    let interval = std::cmp::min(base_interval, MAX_RETRY_INTERVAL);

    // Add some jitter (±20%) to avoid thundering herd problem
    let jitter_factor = 0.8 + (rand::random::<f64>() * 0.4); // Range: 0.8-1.2
    let interval_with_jitter = (interval as f64 * jitter_factor).round() as u64;

    std::cmp::min(interval_with_jitter, MAX_RETRY_INTERVAL)
}

async fn check_update() -> Result<()> {
    let check_update = github::Update::configure()
        .repo_owner("holomotion")
        .repo_name("forwarder")
        .with_fast_git_proxy("https://fastgit.czyt.tech")
        .bin_name("forwarder")
        .show_download_progress(true)
        .current_version(cargo_crate_version!())
        .build();
    match check_update {
        Ok(update) => {
            if let Err(e) = update.update() {
                eprintln!("update failed: {:?}", e);
            }
        }
        Err(e) => {
            eprintln!("check update failed: {:?}", e);
        }
    }
    Ok(())
}

#[cfg(test)]
mod forward_test {
    use anyhow::Result;
    use bore_cli::client;
    use mac_address::{get_mac_address, MacAddressIterator};

    #[warn(dead_code)]
    const NIL_MAC_ADDRESS: &str = "00:00:00:00:00:00";

    #[tokio::test]
    async fn create_bore_client() -> Result<()> {
        let cli = client::Client::new("10.10.68.177", 22, "rustdesk.ntsports.tech", 0, Some("hm#CD888")).await?;
        println!("cli get port:{}", cli.remote_port());
        Ok(())
    }

    #[test]
    fn get_mac_addr() -> Result<()> {
        let addr = get_mac_address()?;
        if let Some(mac) = addr {
            println!("mac addr is {}", mac);
        }
        Ok(())
    }

    #[test]
    fn get_all_mac_addresses() -> Result<()> {
        let mac_iter = MacAddressIterator::new()?;
        let all_mac_addresses: Vec<String> = mac_iter.filter(|ma| ma.to_string() != NIL_MAC_ADDRESS).map(|ma| ma.to_string()).collect();
        println!("mac address:{:?}", all_mac_addresses);
        Ok(())
    }
}