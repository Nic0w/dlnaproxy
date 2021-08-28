extern crate chrono;
extern crate clap;
extern crate ctrlc;
extern crate fern;
extern crate httparse;
extern crate log;
extern crate nix;
extern crate quick_xml;
extern crate reqwest;
extern crate serde;
extern crate timer;
extern crate toml;

mod ssdp;
mod ssdp_broadcast;
mod ssdp_listener;
mod ssdp_packet;
mod ssdp_utils;
mod tcp_proxy;

use std::{
    fs,
    net::{SocketAddr, ToSocketAddrs},
    time,
};

use serde::Deserialize;

use reqwest::Url;

use clap::{App, AppSettings, Arg, ArgMatches};
use log::{debug, trace};

use crate::ssdp::SSDPManager;
use crate::ssdp_utils::Result;
use crate::tcp_proxy::TCPProxy;

#[derive(Deserialize)]
struct RawConfig {
    description_url: Option<String>,
    period: Option<String>,
    proxy: Option<String>,
    verbose: Option<u64>,
    iface: Option<String>,
}

struct Config {
    description_url: Url,
    period: time::Duration,
    proxy: Option<SocketAddr>,
    broadcast_iface: Option<String>,
    verbose: log::LevelFilter,
}

fn main() -> Result<()> {
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
            .required_unless("config"))
        .arg(Arg::with_name("interval")
            .short("d")
            .long("interval")
            .value_name("DURATION")
            .help("Interval at which we will check the remote server's presence and broadcast on its behalf, in seconds.")
            .takes_value(true))
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
        .arg(Arg::with_name("config")
            .short("c")
            .long("config")
            .takes_value(true)
            .conflicts_with_all(&["description-url", "interval", "proxy"])
            .value_name("/path/to/config.conf")
            .help("TOML config file."))
        .arg(Arg::with_name("broadcast-iface")
            .short("i")
            .long("iface")
            .value_name("IFACE")
            .help("Network interface on which to broadcast (requires root or CAP_NET_RAW capability).")
            .takes_value(true))
        .get_matches();

    let config = get_config(args)?;

    init_logging(config.verbose);

    let mut url = config.description_url;

    let _tcp_proxy_thread = if let Some(proxy_addr) = config.proxy {
        let server_addr = sockaddr_from_url(&url);

        url.set_ip_host(proxy_addr.ip()).unwrap();
        url.set_port(Some(proxy_addr.port())).unwrap();

        let proxy = TCPProxy;

        trace!(target: "dlnaproxy", "server: {}", server_addr);

        Some(proxy.start(server_addr, proxy_addr))
    } else {
        None
    };

    debug!(target: "dlnaproxy", "Desc URL: '{}', interval: {}s, verbosity: {}", url, config.period.as_secs(), config.verbose);

    let timeout = time::Duration::from_secs(2);
    let ssdp = SSDPManager::new(
        url.as_str(),
        config.period,
        Some(timeout),
        config.broadcast_iface,
    );
    let (_timer, _guard) = ssdp.start_broadcast();

    ssdp.start_listener().join().expect("Panicked !");

    Ok(())
}

fn get_config(args: ArgMatches) -> Result<Config> {
    let config_as_file = args
        .value_of("config")
        .map(|file| fs::read_to_string(file).map_err(|_| "Could not open/read config file."))
        .transpose()?;

    let raw_config = if let Some(config_file) = config_as_file {
        toml::from_str(&config_file).map_err(|e| {
            eprintln!("{}", e);
            "failed to parse config file."
        })
    } else {
        Ok(RawConfig {
            description_url: args.value_of("description-url").map(|s| s.to_owned()),

            period: args.value_of("interval").map(|s| s.to_owned()),

            proxy: args.value_of("proxy").map(|s| s.to_owned()),

            iface: args.value_of("broadcast-iface").map(|s| s.to_owned()),

            verbose: Some(args.occurrences_of("verbose")),
        })
    }?;

    Ok(Config {
        description_url: raw_config
            .description_url
            .ok_or("Missing description URL")
            .and_then(|s| Url::parse(&s).map_err(|_| "Bad URL."))?,

        period: raw_config
            .period
            .map_or(Ok(895), |v| {
                v.parse::<u64>().map_err(|_| "Bad value for interval.")
            })
            .map(time::Duration::from_secs)?,

        proxy: raw_config
            .proxy
            .map(|s| s.parse().map_err(|_| "Bad address"))
            .transpose()?,

        broadcast_iface: raw_config.iface,

        verbose: raw_config
            .verbose
            .map_or(log::LevelFilter::Warn, |v| match v {
                0 => log::LevelFilter::Warn,
                1 => log::LevelFilter::Info,
                2 => log::LevelFilter::Debug,
                3..=u64::MAX => log::LevelFilter::Trace,
            }),
    })
}

fn sockaddr_from_url(url: &Url) -> SocketAddr {
    let host = url.host().expect("Unsupported URL.");

    let port: u16 = url
        .port_or_known_default()
        .expect("Unknown port or scheme.");

    let address = format!("{}:{}", host, port);

    let addresses: Vec<SocketAddr> = address
        .to_socket_addrs()
        .expect("Couldn't resolve or build socket address from submitted URL.")
        .collect();

    addresses
        .first()
        .expect("No valid socket address.")
        .to_owned()
}

fn init_logging(verbosity: log::LevelFilter) -> log::LevelFilter {
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
        level_for("dlnaproxy", verbosity).
        chain(std::io::stdout()).
        apply().
            expect("Failed to configure logging.");

    verbosity
}
