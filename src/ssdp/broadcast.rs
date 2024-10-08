use log::{debug, info, warn};
use tokio::signal;

use std::{net::UdpSocket, process, sync::Arc};

use anyhow::Result;

use crate::ssdp::utils::InteractiveSSDP;
use crate::ssdp::SSDP_ADDRESS;

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

    pub async fn do_ssdp_alive(&self) -> Result<()> {
        self.ssdp_helper
            .send_alive(&self.ssdp_socket, SSDP_ADDRESS)
            .await
    }
}

pub async fn ctrlc_handler(broadcaster: Arc<SSDPBroadcast>) -> Result<()> {
    debug!(target:"dlnaproxy", "SIGINT handler waiting...");

    signal::ctrl_c().await?;

    let socket = broadcaster.ssdp_socket.try_clone()?;

    let helper = broadcaster.ssdp_helper.clone();

    debug!(target:"dlnaproxy", "SIGINT handler triggered, sending ssdp:bybye !");

    if let Err(msg) = helper.send_byebye(&socket, SSDP_ADDRESS).await {
        warn!(target: "dlnaproxy", "Failed to send ssdp:byebye: {}", msg);
    }

    info!(target: "dlnaproxy", "Exiting !");

    process::exit(0);
}
