use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};

const REPORT_ENDPOINT: &str = "https://rustdesk.ntsports.tech/api/v1/forward_info/update";

#[derive(Serialize, Deserialize, Debug)]
pub struct ForwardEntry {
    pub local_host:String,
    pub local_port: u16,
    pub  remote_port: u16,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ForwardInfo {
    pub app_version:String,
    pub mac_address: String,
    pub forward_entries: Vec<ForwardEntry>,
    pub all_mac_addresses:Vec<String>,
}

impl ForwardInfo{
    pub async fn report(&self ) -> Result<()> {
        let client = &Client::new();
        let user_agent = "holomotion_Forward/1.0.0 (https://holomotion.tech)";
        let content_type = "application/json";
        let response = client.post(REPORT_ENDPOINT)
            .header("User-Agent", user_agent)
            .header("Content-Type", content_type)
            .json(self)
            .send()
            .await?;
        if response.status().is_success() {
            println!("Request succeeded: {:?}", response.text().await?);
        } else {
            println!("Request failed with status: {:?}", response.status());
        }
        Ok(())
    }
}
