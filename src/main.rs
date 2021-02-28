extern crate httparse;
extern crate reqwest;
extern crate serde;
extern crate quick_xml;
extern crate timer;
extern crate chrono;
extern crate clap;
extern crate fern;
extern crate log;
extern crate nix;

mod ssdp_broadcast;
mod ssdp_listener;

use std::{
    process,
    thread,
    time,
    mem,
    sync::Arc,
    os::unix::io::AsRawFd,
    net::{
        UdpSocket,
        Ipv4Addr
    }
};

use clap::{Arg, App, AppSettings};

use log::{info, trace, warn, debug};

use nix::sys::socket::{self, sockopt::ReuseAddr};

use timer::Guard;

use chrono::Utc;

struct ThreadConfig {
    description_url: String,
    ssdp_socket: UdpSocket,
    alive_interval: i64
}

fn main() {

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
            .takes_value(true))
        .arg(Arg::with_name("verbose")
            .short("v")
            .long("verbose")
            .takes_value(false)
            .multiple(true)
            .help("Verbosity level. The more v, the more verbose."))
        .get_matches();

    let verbosity = init_logging(args.occurrences_of("verbose"));

    let url = args.value_of("description-url").
        expect("Missing description URL").to_string();

    let interval: i64 = args.value_of("interval").unwrap_or("300").parse().
            expect("Bad value for interval");

    debug!(target: "dlnaproxy", "Desc URL: '{}', interval: {}s, verbosity: {}", url, interval, verbosity);

    let multicast_addr = Ipv4Addr::new(239, 255, 255, 250);
    let port: u16 = 1900;

    let bind_addr = format!("{addr}:{port}", addr=multicast_addr, port=port);

    let ssdp = UdpSocket::bind(&bind_addr).
        expect("Failed to bind socket");

    socket::setsockopt(ssdp.as_raw_fd(), ReuseAddr, &true).
        expect("Failed to set SO_REUSEADDR.");

    ssdp.join_multicast_v4(&multicast_addr, &Ipv4Addr::UNSPECIFIED).
        expect("Failed to join multicast group");

    let shared_config = Arc::new(ThreadConfig {
        description_url: url,
        ssdp_socket: ssdp,
        alive_interval: interval
    });

    run_listener(shared_config.clone());
    run_broadcast(shared_config.clone());

    //never return!!
    loop {
        thread::sleep(time::Duration::from_millis(100));
    }
}

fn run_listener(config: Arc<ThreadConfig>) {
    thread::spawn(move || {
        if let Ok(socket) = config.ssdp_socket.try_clone() {
            ssdp_listener::do_listen(socket, config.description_url.as_ref());
        }
    });
}

fn run_broadcast(config: Arc<ThreadConfig>) {
    let alive_timer = timer::Timer::new();

    let now = Utc::now();
    let loop_dur = chrono::Duration::seconds(config.alive_interval);

    debug!(target: "dlnaproxy", "About to schedule broadcast every {}s", config.alive_interval);

    let guard = alive_timer.schedule(now, Some(loop_dur), move || {
        trace!(target: "dlnaproxy", "About to attempt to broadcast.");
        if let Ok(socket) = config.ssdp_socket.try_clone() {

            if let Err(msg) = ssdp_broadcast::do_ssdp_alive(socket, config.description_url.as_ref()) {
                warn!(target: "dlnaproxy", "Couldn't send ssdp:alive: {}", msg);
            }
            else {
                info!(target: "dlnaproxy", "Broadcasted on local SSDP channel!");
            }
        }
        else {
            warn!(target: "dlnaproxy", "Broadcast: failed to clone socket.");
        }
    });

    mem::forget(guard);
    mem::forget(alive_timer);
}

fn init_logging(verbosity: u64) -> log::LevelFilter {

    let level: log::LevelFilter = match verbosity {
        0 => log::LevelFilter::Warn,
        1 => log::LevelFilter::Info,
        2 => log::LevelFilter::Debug,
        3..=u64::MAX => log::LevelFilter::Trace
    };

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
        level_for("dlnaproxy", level).
        chain(std::io::stdout()).
        apply().
            expect("Failed to configure logging.");
    level
}
