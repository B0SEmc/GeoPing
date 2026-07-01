use serde::Deserialize;
use std::time::Duration;
use url::Url;

#[derive(Deserialize, Debug, Clone)]
pub struct GeoLocation {
    pub lat: f64,
    pub lon: f64,
    pub city: String,
    pub country_code: String,
}

pub async fn fetch_location(url_str: &str) -> Option<GeoLocation> {
    let host = match Url::parse(url_str) {
        Ok(parsed) => parsed.host_str()?.to_string(),
        Err(_) => return None,
    };

    let url = format!("http://ip-api.com/json/{}?fields=status,message,countryCode,city,lat,lon", host);
    
    #[derive(Deserialize)]
    struct IpApiResponse {
        status: String,
        #[serde(rename = "countryCode")]
        country_code: Option<String>,
        city: Option<String>,
        lat: Option<f64>,
        lon: Option<f64>,
    }

    let resp = reqwest::get(&url).await.ok()?;
    let json = resp.json::<IpApiResponse>().await.ok()?;
    if json.status == "success" {
        return Some(GeoLocation {
            lat: json.lat.unwrap_or(0.0),
            lon: json.lon.unwrap_or(0.0),
            city: json.city.unwrap_or_default(),
            country_code: json.country_code.unwrap_or_default(),
        });
    }
    None
}

pub fn calculate_distance(rtt: Duration) -> f64 {
    // Distance (km) = RTT_ms * 100
    rtt.as_secs_f64() * 1000.0 * 100.0
}

pub fn estimate_location(data: &[(GeoLocation, Duration)]) -> Option<(f64, f64)> {
    if data.is_empty() {
        return None;
    }

    let mut weighted_lat = 0.0;
    let mut weighted_lon = 0.0;
    let mut total_weight = 0.0;

    for (loc, rtt) in data {
        let dist = calculate_distance(*rtt);
        let weight = if dist > 0.0 { 1.0 / dist } else { 1.0 };
        
        weighted_lat += loc.lat * weight;
        weighted_lon += loc.lon * weight;
        total_weight += weight;
    }

    if total_weight > 0.0 {
        Some((weighted_lat / total_weight, weighted_lon / total_weight))
    } else {
        None
    }
}
