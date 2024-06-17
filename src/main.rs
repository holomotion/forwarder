mod reporter;

use bore_cli::client;
use anyhow::Result;
use mac_address::{get_mac_address, MacAddressIterator};
use crate::reporter::{ForwardEntry, ForwardInfo};


const LOCALHOST: &str = "localhost";
const FORWARD_SERVER: &str = "rustdesk.ntsports.tech";
const FORWARD_SECRET: &str = "hm#CD888";
const NIL_MAC_ADDRESS:&str =  "00:00:00:00:00:00";

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

        let ssh_forward = tokio::spawn(
            ssh_cli.listen()
        );
        let cockpit_forward = tokio::spawn(
            cockpit_cli.listen()
        );
        _ = tokio::join!(
            ssh_forward,
            cockpit_forward,
        );
    }
    Ok(())
}

#[cfg(test)]
mod forward_test {
    use bore_cli::client;
    use anyhow::Result;
    use mac_address::{get_mac_address, MacAddressIterator};

    #[warn(dead_code)]
    const NIL_MAC_ADDRESS:&str =  "00:00:00:00:00:00";
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
        let all_mac_addresses:Vec<String> = mac_iter.filter(|ma| ma.to_string() != NIL_MAC_ADDRESS).map(|ma| ma.to_string()).collect();
        println!("mac address:{:?}",all_mac_addresses);
        Ok(())
    }
}
