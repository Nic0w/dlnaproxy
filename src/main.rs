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

mod ssdp_packet;
mod ssdp_utils;
mod ssdp_broadcast;
mod ssdp_listener;
mod ssdp;
mod tcp_proxy;


use std::{
    time,
    net::{
        SocketAddr,
        ToSocketAddrs
    }
};

use reqwest::Url;

use clap::{Arg, App, AppSettings};
use log::{info, trace, warn, debug};


use crate::ssdp::SSDPManager;
use crate::tcp_proxy::TCPProxy;

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
        /*.arg(Arg::with_name("repeater")
            .short("r")
            .long("repeater")
            .takes_value(false)
            .help("Disable proxy mode. The description URL will be broadcasted as is on the local network. Some DLNA devices don't like that."))*/
        .arg(Arg::with_name("proxy")
            .short("p")
            .long("proxy")
            .takes_value(true)
            .value_name("IP:PORT")
            .help("IP address to bind the proxy on."))
        .arg(Arg::with_name("verbose")
            .short("v")
            .long("verbose")
            .takes_value(false)
            .multiple(true)
            .help("Verbosity level. The more v, the more verbose."))
        .get_matches();

    let verbosity = init_logging(args.occurrences_of("verbose"));

    let mut url = args.value_of("description-url")
        .map(|url| Url::parse(url).expect("Bad URL."))
        .expect("Missing description URL");

    let interval: u64 = args.value_of("interval").unwrap_or("895").parse().
            expect("Bad value for interval");

    //let repeater_mode = args.is_present("repeater");

    let parsed_addr : Option<SocketAddr> = args.value_of("proxy").
        map(|addr| addr.parse().expect("Bad proxy address"));

    let tcp_proxy_thread = if let Some(proxy_addr) = parsed_addr {

        let server_addr = sockaddr_from_url(&url);

        url.set_ip_host(proxy_addr.ip()).
            unwrap();
        url.set_port(Some(proxy_addr.port())).
        unwrap();

        let proxy = TCPProxy;

        trace!(target: "dlnaproxy", "server: {}", server_addr);

        Some(proxy.start(server_addr, proxy_addr))
    }
    else { None };

    debug!(target: "dlnaproxy", "Desc URL: '{}', interval: {}s, verbosity: {}", url, interval, verbosity);


    let period = time::Duration::from_secs(interval);
    let timeout = time::Duration::from_secs(2);
    let ssdp = SSDPManager::new(url.as_str(), period, Some(timeout));
    let (timer, guard) = ssdp.start_broadcast();

    ssdp.start_listener().join().
        expect("Panicked !");
}

fn sockaddr_from_url(url: &Url) -> SocketAddr {
    let host = url.host().
        expect("Unsupported URL.");

    let port: u16 = url.port_or_known_default().
        expect("Unknown port or scheme.");

    let address = format!("{}:{}", host, port);

    let addresses: Vec<SocketAddr> = address.to_socket_addrs().
        expect("Couldn't resolve or build socket address from submitted URL.").
        collect();

    addresses.first().
        expect("No valid socket address.").
        to_owned()
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
