use log::{debug, info, warn};

use std::{net::UdpSocket, process, sync::Arc};

use crate::ssdp::SSDP_ADDRESS;
use crate::ssdp::utils::{InteractiveSSDP, Result};

pub struct SSDPBroadcast {
    ssdp_socket: UdpSocket,
    ssdp_helper: Arc<InteractiveSSDP>,
}

impl SSDPBroadcast {
    pub fn new(ssdp_socket: UdpSocket, ssdp_helper: Arc<InteractiveSSDP>) -> Self {
        SSDPBroadcast {
            ssdp_socket,
            ssdp_helper,
        }
    }

    pub fn sigint_handler(&self) -> impl FnMut() {
        let socket = self.ssdp_socket.try_clone().unwrap();

        let helper = self.ssdp_helper.clone();

        move || {
            debug!(target:"dlnaproxy", "SIGINT handler triggered, sending ssdp:bybye !");

            if let Err(msg) = helper.send_byebye(&socket, SSDP_ADDRESS) {
                warn!(target: "dlnaproxy", "Failed to send ssdp:byebye: {}", msg);
            }

            info!(target: "dlnaproxy", "Exiting !");

            process::exit(0);
        }
    }

    pub fn do_ssdp_alive(&self) -> Result<()> {
        self.ssdp_helper.send_alive(&self.ssdp_socket, SSDP_ADDRESS)
    }
}
