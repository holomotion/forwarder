use bore_cli::client;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let ssh_cli = client::Client::new("10.10.68.177", 22, "bore.czyt.tech", 0, Some("rustean")).await?;
    println!("cli get port:{}", ssh_cli.remote_port());
    // start a thread to run client
    let ssh_forward = tokio::spawn(
        ssh_cli.listen()
    );
    _ = tokio::join!(ssh_forward);
    Ok(())
}

#[cfg(test)]
mod forward_test {
    use bore_cli::client;
    use anyhow::Result;
    use mac_address::get_mac_address;

    #[tokio::test]
    async fn create_bore_client() -> Result<()> {
        let cli = client::Client::new("10.10.68.177", 22, "bore.czyt.tech", 0, Some("rustean")).await?;
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
}
