extern crate httparse;
extern crate reqwest;
extern crate serde;
extern crate quick_xml;
extern crate timer;
extern crate chrono;
extern crate clap;
extern crate syslog;
extern crate fern;
extern crate log;

mod ssdp_alive;

use std::{
    process,
    sync::Arc,
    net::{
        UdpSocket,
        Ipv4Addr
    }
};
use log::{info, trace, warn, debug};

use syslog::Facility;

use chrono::Utc;
use clap::{Arg, App, AppSettings};

fn main() {

    init_logging();

    let args = App::new("DLNAProxy")
        .setting(AppSettings::ArgRequiredElseHelp)
        .version("1.0")
        .author("Nic0w")
        .about("Broadcast ssdp:alive messages on the local network's multicast SSDP channel on behalf of a remote DLNA server.")
        .arg(Arg::with_name("description-url")
            .short("u")
            .long("desc-url")
            .value_name("URL")
            .help("URL pointing to the remote DLNA server's root XML description.")
            .takes_value(true)
            .required(true))
        .arg(Arg::with_name("interval")
            .short("i")
            .long("interval")
            .value_name("DURATION")
            .help("Interval at which we will check the remote server's presence and broadcast on its behalf, in seconds.")
            .takes_value(true)).
        get_matches();

    let url = args.value_of("description-url").
        expect("Missing description URL");

    let interval: i64 = args.value_of("interval").unwrap_or("300").parse().
            expect("Bad value for interval");

    info!(target: "dlnaproxy", "Started with URL {} and interval: {}s", url, interval);

    let multicast_addr = Ipv4Addr::new(239, 255, 255, 250);
    let port: u16 = 1900;

    let bind_addr = format!("{addr}:{port}", addr=multicast_addr, port=port);

    let ssdp = UdpSocket::bind(&bind_addr).
        expect("Failed to bind socket");

    ssdp.join_multicast_v4(&multicast_addr, &Ipv4Addr::UNSPECIFIED).
        expect("Failed to join multicast group");

    run(ssdp, url, interval);
}

fn run(ssdp: UdpSocket, url: &str, interval: i64) {
    let alive_timer = timer::Timer::new();

    let now = Utc::now();
    let loop_dur = chrono::Duration::seconds(interval);

    let safe_url = Arc::new(url.to_owned());

    let _guard = alive_timer.schedule(now, Some(loop_dur), move || {
        if let Ok(socket) = ssdp.try_clone() {

            if let Err(msg) = ssdp_alive::do_ssdp_alive(socket, safe_url.as_ref()) {
                eprintln!("Fail: {}", msg);
            }
        }
        else {
            eprintln!("Failed to clone socket.");
        }
    });
    loop {}
}

fn init_logging() {

    /*let formatter = syslog::Formatter3164 {
            facility: Facility::LOG_USER,
            hostname: None,
            process: "dlnaproxy".into(),
            pid: process::id() as i32
    };

    let syslog_binding = syslog::unix(formatter).
        expect("Failed to connect to syslog.");*/

    fern::Dispatch::new().
        format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        }).
        // by default only accept warning messages from libraries so we don't spam
        level(log::LevelFilter::Warn).
        // but accept Info and Debug and Trace for our app.
        level_for("dlnaproxy", log::LevelFilter::Trace).
        chain(std::io::stdout()).
        apply().
            expect("Failed to configure logging.");
}
