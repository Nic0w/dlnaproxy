use std::net::UdpSocket;
use serde::Deserialize;
use reqwest::{
    blocking::Response,
    header::SERVER
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

struct EndpointInfo {
    device_type: String,
    unique_device_name: String,
    server: Option<String>
}

type Result<T> =  std::result::Result<T, &'static str>;

fn fetch_endpoint_info(url: &str) -> Result<EndpointInfo> {

    let endpoint_response = reqwest::blocking::get(url).
        map_err(|_| "Failed to get description of remote endpoint.")?;

    let server_ua = endpoint_response.headers().get(SERVER).
        map(|hv| String::from_utf8_lossy(hv.as_bytes()).to_string());

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


pub fn do_ssdp_alive(ssdp_socket: UdpSocket, endpoint_desc_url: &str) -> Result<()> {

    let endpoint_info = fetch_endpoint_info(endpoint_desc_url)?;

    let default_ua = "DLNAProxy/1.0".to_string();
    let user_agent = endpoint_info.server.or(Some(default_ua));

    let ssdp_alive = format!("
NOTIFY * HTTP/1.1\r\n\
HOST:239.255.255.250:1900\r\n\
CACHE-CONTROL:max-age={cache_max_age}\r\n\
LOCATION:{location}\r\n\
SERVER: {server_ua}\r\n\
NT:{device_type}\r\n\
USN:{udn}::{device_type}\r\n\
NTS:ssdp:alive\r\n\
\r\n",
    cache_max_age=130, location=endpoint_desc_url, server_ua=user_agent.unwrap(), device_type=endpoint_info.device_type, udn=endpoint_info.unique_device_name);

    ssdp_socket.send_to(ssdp_alive.as_bytes(), "239.255.255.250:1900");

    Ok(println!("{}", ssdp_alive))
}
