extern crate httparse;
extern crate reqwest;
extern crate serde;
extern crate quick_xml;
extern crate timer;
extern crate chrono;

mod ssdp;
mod ssdp_alive;

use std::net::{ UdpSocket, Ipv4Addr };
use std::thread;

use chrono::{ DateTime, Utc};

fn main() {
    println!("Hello, world!");

    let multicast_addr = Ipv4Addr::new(239, 255, 255, 250);
    let port: u16 = 1900;

    let bind_addr = format!("{addr}:{port}", addr=multicast_addr, port=port);

    let ssdp = UdpSocket::bind(&bind_addr).
        expect("Failed to bind socket.");

    ssdp.join_multicast_v4(&multicast_addr, &Ipv4Addr::UNSPECIFIED).
        expect("Failed to join multicast group");

    let url = "http://10.0.42.1:8200/rootDesc.xml";

    let alive_timer = timer::Timer::new();


    let now = Utc::now();
    let loop_dur = chrono::Duration::seconds(10);

    let _guard = alive_timer.schedule(now, Some(loop_dur), move || {
        if let Ok(socket) = ssdp.try_clone() {

            if let Err(msg) = ssdp_alive::do_ssdp_alive(socket, url) {
                eprintln!("Fail: {}", msg);
            }
        }
        else {
            eprintln!("Failed to clone socket.");
        }
    });

    thread::sleep(std::time::Duration::from_millis(60*1000));
}
