use serde::{Deserialize, Serialize};
use anyhow::Result;
use reqwest::Client;


const REPORT_ENDPOINT: &str = "https://rustdesk.ntsports.tech/api/v1/forward_info/update";

#[derive(Serialize, Deserialize, Debug)]
pub struct ForwardEntry {
    pub local_host:&'static str,
    pub local_port: u16,
    pub  remote_port: u16,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ForwardInfo {
    pub mac_address: String,
    pub forward_entries: Vec<ForwardEntry>,
}

impl ForwardInfo{
    pub async fn report(&self) -> Result<(), Box<dyn std::error::Error>> {
        let client = Client::new();
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