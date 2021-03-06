use std:: {
    thread,
    sync::Arc,
    time::Duration,
    os::unix::io::AsRawFd,
    net::{
        Ipv4Addr,
        UdpSocket,
    }
};

use log::{info, trace, warn, debug};

use chrono::Utc;
use nix::sys::socket::{self, sockopt::ReuseAddr};


use crate::ssdp_broadcast::SSDPBroadcast;
use crate::ssdp_listener::SSDPListener;

static SSDP_ADDRESS: (Ipv4Addr, u16) = (Ipv4Addr::new(239, 255, 255, 250), 1900);

pub struct SSDPManager {
    broadcast_period: Duration,
    broadcaster: Arc<SSDPBroadcast>,
    timer: Option<(timer::Timer, timer::Guard)>,

    listener: Arc<SSDPListener>,
}

impl SSDPManager {

    pub fn new(endpoint_desc_url: &str, broadcast_period: Duration, connect_timeout: Option<Duration>) -> Self {

        let http_client = reqwest::blocking::Client::builder().
                connect_timeout(connect_timeout).
                build().
                    expect("Failed to build HTTP client.");

        let (ssdp1, ssdp2) = ssdp_socket_pair();

        let ssdp_broadcast = Arc::new(
            SSDPBroadcast::new(ssdp1, http_client, endpoint_desc_url)
        );

        let broadcast_for_listener = ssdp_broadcast.clone();

        let ssdp_listener = Arc::new(
            SSDPListener::new(ssdp2, broadcast_for_listener)
        );

        SSDPManager {
            broadcast_period: broadcast_period,
            timer: None,

            broadcaster: ssdp_broadcast,
            listener: ssdp_listener
        }
    }

    pub fn start_broadcast(&mut self) {
        let broadcast = self.broadcaster.clone();

        let alive_timer = timer::Timer::new();

        let now = Utc::now();
        let period = chrono::Duration::from_std(self.broadcast_period).
            expect("Too large period.");

        debug!(target: "dlnaproxy", "About to schedule broadcast every {}s", period.num_seconds());

        let guard = alive_timer.schedule(now, Some(period), move || {
            if let Err(msg) = broadcast.do_ssdp_alive()  {
                warn!(target: "dlnaproxy", "Couldn't send ssdp:alive: {}", msg);
            }
            else {
                info!(target: "dlnaproxy", "Broadcasted on local SSDP channel!");
            }
        });

        self.timer = Some((alive_timer, guard));
    }

    pub fn start_listener(&self) {
        let listener = self.listener.clone();
        thread::spawn(move || {
            listener.do_listen();
        });
    }

}

fn ssdp_socket_pair() -> (UdpSocket, UdpSocket) {

    let bind_addr = format!("{addr}:{port}", addr=SSDP_ADDRESS.0, port=SSDP_ADDRESS.1);

    let ssdp1 = UdpSocket::bind(&bind_addr).
        expect("Failed to bind socket");

    socket::setsockopt(ssdp1.as_raw_fd(), ReuseAddr, &true).
        expect("Failed to set SO_REUSEADDR.");

    ssdp1.join_multicast_v4(&SSDP_ADDRESS.0, &Ipv4Addr::UNSPECIFIED).
        expect("Failed to join multicast group");

    let ssdp2 = ssdp1.try_clone().
        expect("Failed to clone SSDP socket.");

    (ssdp1, ssdp2)
}
