use duct::cmd;
use anyhow::Result;
pub(crate) fn get_access_id() -> Result<String> {
    let access_id = cmd!("rustdesk","--get-id").read().unwrap_or("".to_string());
    Ok(access_id)
}