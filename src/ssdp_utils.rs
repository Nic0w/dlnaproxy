
use serde::Deserialize;
use reqwest::{
    header::SERVER,
    blocking
};

#[derive(Debug, Deserialize)]
struct DLNADevice {
    #[serde(rename = "deviceType")]
    device_type: String,

    #[serde(rename = "UDN")]
    unique_device_name: String,
}

#[derive(Debug, Deserialize)]
struct DLNADescription {
    device: DLNADevice
}

pub struct EndpointInfo {
    pub device_type: String,
    pub unique_device_name: String,
    pub server: String
}

pub type Result<T> =  std::result::Result<T, &'static str>;

pub fn fetch_endpoint_info(http: &blocking::Client, url: &str) -> Result<EndpointInfo> {

    let endpoint_response = http.get(url).send().
        map_err(|_| "Failed to get description of remote endpoint.")?;

    let server_ua = endpoint_response.headers().get(SERVER).
        map(|hv| String::from_utf8_lossy(hv.as_bytes()).to_string()).
        unwrap_or("DLNAProxy/1.0".into());

    let body = endpoint_response.text().
        map_err(|_| "Failed to parse response's body as text.")?;

    let device_description: DLNADescription = quick_xml::de::from_str(&body).
        map_err(|_| "Failed to parse device's XML description.")?;

    Ok(EndpointInfo {
        device_type: device_description.device.device_type,
        unique_device_name: device_description.device.unique_device_name,
        server: server_ua
    })
}
