use std::time::Duration;

use anyhow::Result;
use bore_cli::client;
use mac_address::{get_mac_address, MacAddressIterator};
use self_github_update::{backends::github, cargo_crate_version};
use tokio::time::timeout;

use crate::reporter::{ForwardEntry, ForwardInfo};

mod reporter;

const LOCALHOST: &str = "localhost";
const FORWARD_SERVER: &str = "rustdesk.ntsports.tech";
const FORWARD_SECRET: &str = "hm#CD888";
const NIL_MAC_ADDRESS: &str = "00:00:00:00:00:00";

#[tokio::main]
async fn main() -> Result<()> {
    let mac_address_result = get_mac_address()?;
    if let Some(mac_address) = mac_address_result {
        let ssh_cli = client::Client::new(LOCALHOST, 22, FORWARD_SERVER, 0, Some(FORWARD_SECRET)).await?;
        let cockpit_cli = client::Client::new(LOCALHOST, 9090, FORWARD_SERVER, 0, Some(FORWARD_SECRET)).await?;
        let all_mac_address_iter = MacAddressIterator::new()?;
        let all_mac_addresses = all_mac_address_iter.filter(|ma| ma.to_string() != NIL_MAC_ADDRESS).map(|ma| ma.to_string()).collect();
        // create forward info
        let forward_info = &ForwardInfo {
            app_version: cargo_crate_version!().parse()?,
            mac_address: mac_address.to_string(),
            forward_entries: vec![
                ForwardEntry {
                    local_host: LOCALHOST.parse()?,
                    local_port: 22,
                    remote_port: ssh_cli.remote_port(),
                },
                ForwardEntry {
                    local_host: LOCALHOST.parse()?,
                    local_port: 9090,
                    remote_port: cockpit_cli.remote_port(),
                },
            ],
            all_mac_addresses,
        };
        forward_info.report().await?;

        let check_update = tokio::spawn(
            timeout(Duration::from_secs(300),  check_update())
        );
        let ssh_forward = tokio::spawn(
            ssh_cli.listen()
        );
        let cockpit_forward = tokio::spawn(
            cockpit_cli.listen()
        );
        _ = tokio::join!(
            ssh_forward,
            cockpit_forward,
            check_update,
        );
    }
    Ok(())
}

async fn  check_update()->Result<()> {
    let check_update = github::Update::configure()
        .repo_owner("holomotion")
        .repo_name("forwarder")
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
            println!("mac addr is {}", mac.to_string())
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
