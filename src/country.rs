use anyhow::Error;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Country {
    pub ip: String,
    pub country: String,
}

pub async fn get_current_country() -> Result<Country, Error> {
    let country = reqwest::get("https://api.country.is")
        .await?
        .json::<Country>()
        .await?;
    Ok(country)
}
#[cfg(test)]
mod country_test {
    use crate::country::get_current_country;

    #[tokio::test]
    async fn test_get_current_country(){
       let country =  get_current_country().await;
        match country {
            Ok(_country) => {
                println!("{:?}",_country)
            }
            Err(e) => {
                println!("{:?}",e)
            }
        }
    }

}