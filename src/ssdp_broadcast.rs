use log::{info, trace, warn, debug};

use std::net::UdpSocket;
use std::sync::Arc;
use reqwest::blocking;

use crate::ssdp_packet::SSDPPacket;
use crate::ssdp_utils::{ self, Result };

pub struct SSDPBroadcast {
    ssdp_socket: UdpSocket,
    http_client: Arc<blocking::Client>,
    desc_url: String
}

impl SSDPBroadcast {

    pub fn new(ssdp_socket: UdpSocket, http_client: Arc<blocking::Client>, desc_url: &str) -> Self {
        SSDPBroadcast {
            ssdp_socket: ssdp_socket,
            http_client: http_client,
            desc_url: desc_url.into()
        }
    }

    pub fn do_ssdp_alive(&self) -> Result<()> {

        trace!(target: "dlnaproxy", "Fetching remote server's info.");
        let endpoint_info = ssdp_utils::fetch_endpoint_info(&self.http_client, &self.desc_url)?;

        let ssdp_alive = SSDPPacket::Alive {
            desc_url: self.desc_url.clone(),
            server_ua: endpoint_info.server,
            device_type: endpoint_info.device_type,
            unique_device_name: endpoint_info.unique_device_name
        };

        trace!(target: "dlnaproxy", "{}", ssdp_alive.to_string());

        ssdp_alive.send_to(&self.ssdp_socket, "239.255.255.250:1900")
            .map_err(|_| "Failed to send on UDP socket")?;

        Ok(debug!(target: "dlnaproxy", "Sent ssdp:alive packet !"))
    }
}
