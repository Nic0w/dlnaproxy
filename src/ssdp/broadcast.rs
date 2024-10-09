use log::{debug, info, warn};
use tokio::net::UdpSocket;
use tokio::{signal, time};

use std::borrow::Borrow as _;
use std::time::Duration;
use std::{process, sync::Arc};

use anyhow::Result;

use crate::ssdp::utils::InteractiveSSDP;
use crate::ssdp::SSDP_ADDRESS;

pub struct SSDPBroadcast {
    ssdp_socket: Arc<UdpSocket>,
    ssdp_helper: Arc<InteractiveSSDP>,
}

impl SSDPBroadcast {
    pub fn new(ssdp_socket: Arc<UdpSocket>, ssdp_helper: Arc<InteractiveSSDP>) -> Self {
        SSDPBroadcast {
            ssdp_socket,
            ssdp_helper,
        }
    }

    pub async fn do_ssdp_alive(&self) -> Result<()> {
        self.ssdp_helper
            .send_alive(self.ssdp_socket.borrow(), SSDP_ADDRESS)
            .await
    }
}

pub async fn broadcast_task(broadcaster: Arc<SSDPBroadcast>, period: Duration) {
    let _handle = tokio::spawn(ctrlc_handler(broadcaster.clone()));

    debug!(target: "dlnaproxy", "About to schedule broadcast every {}s", period.as_secs());

    let mut interval = time::interval(period);

    loop {
        if let Err(msg) = broadcaster.do_ssdp_alive().await {
            warn!(target: "dlnaproxy", "Couldn't send ssdp:alive: {}", msg);
            break;
        } else {
            info!(target: "dlnaproxy", "Broadcasted on local SSDP channel!");
        }

        interval.tick().await;
    }
}

pub async fn ctrlc_handler(broadcaster: Arc<SSDPBroadcast>) -> Result<()> {
    debug!(target:"dlnaproxy", "SIGINT handler waiting...");

    signal::ctrl_c().await?;

    let socket = broadcaster.ssdp_socket.clone();

    let helper = broadcaster.ssdp_helper.clone();

    debug!(target:"dlnaproxy", "SIGINT handler triggered, sending ssdp:bybye !");

    if let Err(msg) = helper.send_byebye(&socket, SSDP_ADDRESS).await {
        warn!(target: "dlnaproxy", "Failed to send ssdp:byebye: {}", msg);
    }

    info!(target: "dlnaproxy", "Exiting !");

    process::exit(0);
}
