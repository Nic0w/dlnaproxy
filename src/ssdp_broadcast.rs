use std::net::UdpSocket;
use std::sync::Arc;

use crate::ssdp_utils::{ Result, InteractiveSSDP };
use crate::ssdp::SSDP_ADDRESS;

pub struct SSDPBroadcast {
    ssdp_socket: UdpSocket,
    ssdp_helper: Arc<InteractiveSSDP>,
}

impl SSDPBroadcast {

    pub fn new(ssdp_socket: UdpSocket, ssdp_helper: Arc<InteractiveSSDP>) -> Self {
        SSDPBroadcast {
            ssdp_socket: ssdp_socket,
            ssdp_helper: ssdp_helper
        }
    }

    pub fn do_ssdp_alive(&self) -> Result<()> {
        self.ssdp_helper.send_alive(&self.ssdp_socket, SSDP_ADDRESS)
    }
}
