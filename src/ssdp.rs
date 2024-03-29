use reqwest::blocking;
use std::{
    net::{Ipv4Addr, UdpSocket},
    os::unix::io::AsRawFd,
    sync::Arc,
    thread::{self, JoinHandle},
    time::Duration,
};

use log::{debug, info, warn};

use chrono::Utc;

#[cfg(any(target_os = "android", target_os = "linux"))]
use nix::sys::socket::sockopt::BindToDevice;

use nix::sys::socket::{self, sockopt::ReuseAddr, SetSockOpt};

use crate::ssdp_broadcast::SSDPBroadcast;
use crate::ssdp_listener::SSDPListener;
use crate::ssdp_utils::InteractiveSSDP;

pub static SSDP_ADDRESS: (Ipv4Addr, u16) = (Ipv4Addr::new(239, 255, 255, 250), 1900);

pub struct SSDPManager {
    broadcast_period: Duration,
    listener: Arc<SSDPListener>,
    broadcaster: Arc<SSDPBroadcast>,
}

impl SSDPManager {
    pub fn new(
        endpoint_desc_url: &str,
        broadcast_period: Duration,
        connect_timeout: Option<Duration>,
        broadcast_iface: Option<String>,
    ) -> Self {
        let http_client = blocking::Client::builder()
            .connect_timeout(connect_timeout)
            .build()
            .expect("Failed to build HTTP client.");

        let (ssdp1, ssdp2) = ssdp_socket_pair(broadcast_iface);

        let cache_max_age = match broadcast_period.as_secs() {
            n if n < 20 => 20,
            n => (n * 2),
        } as usize;

        let interactive_ssdp = Arc::new(InteractiveSSDP::new(
            http_client,
            endpoint_desc_url,
            cache_max_age,
        ));

        //We send an initial byebye before all else because... that's how MiniDLNA does it.
        //Guessing that it's for clearing any cache that might exist on listening remote devices.
        interactive_ssdp
            .send_byebye(&ssdp1, SSDP_ADDRESS)
            .expect("Failed to send initial ssdp:byebye !");

        let listener = Arc::new(SSDPListener::new(ssdp1, interactive_ssdp.clone()));

        let broadcaster = Arc::new(SSDPBroadcast::new(ssdp2, interactive_ssdp));

        SSDPManager {
            broadcast_period,
            listener,
            broadcaster,
        }
    }

    pub fn start_broadcast(&self) -> (timer::Timer, timer::Guard) {
        let broadcast = self.broadcaster.clone();

        let alive_timer = timer::Timer::new();

        let now = Utc::now();
        let period = chrono::Duration::from_std(self.broadcast_period).expect("Too large period.");

        ctrlc::set_handler(broadcast.sigint_handler()).expect("Failed to set SIGING handler.");

        debug!(target: "dlnaproxy", "About to schedule broadcast every {}s", period.num_seconds());

        let guard = alive_timer.schedule(now, Some(period), move || {
            if let Err(msg) = broadcast.do_ssdp_alive() {
                warn!(target: "dlnaproxy", "Couldn't send ssdp:alive: {}", msg);
            } else {
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

fn ssdp_socket_pair(broadcast_iface: Option<String>) -> (UdpSocket, UdpSocket) {
    let ssdp1 = UdpSocket::bind(SSDP_ADDRESS).expect("Failed to bind socket");

    socket::setsockopt(ssdp1.as_raw_fd(), ReuseAddr, &true).expect("Failed to set SO_REUSEADDR.");

    if let Some(iface) = broadcast_iface {
        let iface = std::ffi::OsString::from(iface);

        #[cfg(any(target_os = "android", target_os = "linux"))]
        socket::setsockopt(ssdp1.as_raw_fd(), BindToDevice, &iface)
            .expect("Failed to set SO_BINDTODEVICE.");

        #[cfg(any(target_os = "macos"))]
        panic!("Cannot set broadcast address on MacOS (yet)")
    }

    ssdp1
        .join_multicast_v4(&SSDP_ADDRESS.0, &Ipv4Addr::UNSPECIFIED)
        .expect("Failed to join multicast group");

    let ssdp2 = ssdp1.try_clone().expect("Failed to clone SSDP socket.");

    (ssdp1, ssdp2)
}
