use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct GeoIpResponse {
    country_code: String,
    region: Option<String>,
}

pub async fn detect_region() -> Result<String> {
    let resp = reqwest::get("https://api.ip.sb/geoip")
        .await
        .context("failed to query ip.sb")?
        .json::<GeoIpResponse>()
        .await
        .context("failed to parse geoip response")?;

    Ok(map_to_region(&resp.country_code, resp.region.as_deref()))
}

fn map_to_region(country: &str, region: Option<&str>) -> String {
    match country {
        "CN" => match region {
            Some(r) if r.contains("Beijing") || r.contains("Hebei") || r.contains("Tianjin") => "cn-north",
            Some(r) if r.contains("Shanghai") || r.contains("Jiangsu") || r.contains("Zhejiang") => "cn-east",
            Some(r) if r.contains("Guangdong") || r.contains("Fujian") => "cn-south",
            _ => "cn-central",
        },
        "US" => match region {
            Some(r) if r.contains("California") || r.contains("Oregon") || r.contains("Washington") => "us-west",
            Some(r) if r.contains("New York") || r.contains("Virginia") => "us-east",
            _ => "us-central",
        },
        "JP" => "jp",
        "KR" => "kr",
        "SG" => "sg",
        "DE" | "FR" | "GB" | "NL" => "eu-west",
        _ => "default",
    }.to_string()
}
