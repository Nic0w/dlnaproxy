use std:: {
    thread::{
        self,
        JoinHandle
    },
    sync::Arc,
    time::Duration,
    os::unix::io::AsRawFd,
    net::{
        Ipv4Addr,
        UdpSocket,
    }
};
use reqwest::blocking;

use log::{info, trace, warn, debug};

use chrono::Utc;
use nix::sys::socket::{self, sockopt::ReuseAddr};


use crate::ssdp_utils::InteractiveSSDP;
use crate::ssdp_listener::SSDPListener;
use crate::ssdp_broadcast::SSDPBroadcast;

static SSDP_ADDRESS: (Ipv4Addr, u16) = (Ipv4Addr::new(239, 255, 255, 250), 1900);

pub struct SSDPManager {
    broadcast_period: Duration,
    listener: Arc<SSDPListener>,
    broadcaster: Arc<SSDPBroadcast>
}

impl SSDPManager {

    pub fn new(endpoint_desc_url: &str, broadcast_period: Duration, connect_timeout: Option<Duration>) -> Self {

        let http_client = blocking::Client::builder().
                connect_timeout(connect_timeout).
                build().
                    expect("Failed to build HTTP client.");

        let (ssdp1, ssdp2) = ssdp_socket_pair();

        let cache_max_age = match broadcast_period.as_secs() {
            n if n < 20 => 20 as usize,
            n => (n * 2) as usize
        };

        let interactive_ssdp = Arc::new(
            InteractiveSSDP::new(http_client, endpoint_desc_url, cache_max_age)
        );

        let listener = Arc::new(
            SSDPListener::new(ssdp1, interactive_ssdp.clone())
        );

        let broadcaster = Arc::new(
            SSDPBroadcast::new(ssdp2, interactive_ssdp.clone())
        );

        SSDPManager {
            broadcast_period: broadcast_period,
            listener: listener,
            broadcaster: broadcaster
        }
    }

    pub fn start_broadcast(&self) -> (timer::Timer, timer::Guard) {
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

        (alive_timer, guard)
    }

    pub fn start_listener(&self) -> JoinHandle<()> {
        let listener = self.listener.clone();

        thread::spawn(move || {
            listener.do_listen();
        })
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
