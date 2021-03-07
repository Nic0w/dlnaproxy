use log::{info, trace, warn, debug};

use std::net::ToSocketAddrs;
use std::net::UdpSocket;

use serde::Deserialize;
use reqwest::{
    header::SERVER,
    blocking
};

use crate::ssdp_packet::SSDPPacket;

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

pub struct InteractiveSSDP {
    http_client: blocking::Client,
    remote_desc_url: String,
    cache_max_age: usize
}

impl InteractiveSSDP {

    pub fn new(client: blocking::Client, url: &str, cache_max_age: usize) -> Self {
        InteractiveSSDP {
                http_client: client,
                remote_desc_url: url.into(),
                cache_max_age: cache_max_age
        }
    }

    fn fetch_endpoint_info(&self) -> Result<EndpointInfo> {

        trace!(target: "dlnaproxy", "Fetching remote server's info.");

        let endpoint_response = self.http_client.get(&self.remote_desc_url).send().
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

    fn send_to(&self, socket: &UdpSocket, dest: impl ToSocketAddrs, ssdp_packet: SSDPPacket, p_type: &str)-> Result<()> {

        trace!(target: "dlnaproxy", "{}", ssdp_packet.to_string());

        ssdp_packet.send_to(socket, dest)?;

        Ok(debug!(target: "dlnaproxy", "Sent ssdp:{} packet !", p_type))
    }

    pub fn send_alive(&self, socket: &UdpSocket, dest: impl ToSocketAddrs) -> Result<()> {

        let info = self.fetch_endpoint_info()?;

        let ssdp_alive = SSDPPacket::Alive {
            desc_url: self.remote_desc_url.clone(),
            server_ua: info.server,
            device_type: info.device_type,
            unique_device_name: info.unique_device_name,
            cache_max_age: self.cache_max_age
        };

        self.send_to(socket, dest, ssdp_alive, "alive")
    }

    pub fn send_ok(&self, socket: &UdpSocket, dest: impl ToSocketAddrs) -> Result<()> {
        let info = self.fetch_endpoint_info()?;

        let ssdp_ok = SSDPPacket::Ok {
            desc_url: self.remote_desc_url.clone(),
            unique_device_name: info.unique_device_name,
            device_type: info.device_type,
            server_ua: info.server,
            cache_max_age: self.cache_max_age
        };

        self.send_to(socket, dest, ssdp_ok, "ok")
    }

    pub fn send_byebye(&self, _socket: &UdpSocket, _dest: impl ToSocketAddrs) -> Result<()> {
        Err("Not implemented!")
    }
}
