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
    time, path::PathBuf,
};

use serde::Deserialize;

use reqwest::Url;

use clap::{Parser, ArgAction};
use log::{debug, trace};

use crate::ssdp::SSDPManager;
use crate::ssdp_utils::Result;
use crate::tcp_proxy::TCPProxy;

#[derive(Deserialize)]
struct RawConfig {
    description_url: Option<String>,
    period: Option<u64>,
    proxy: Option<String>,
    verbose: Option<usize>,
    iface: Option<String>,
}

struct Config {
    description_url: Url,
    period: time::Duration,
    proxy: Option<SocketAddr>,
    broadcast_iface: Option<String>,
    verbose: log::LevelFilter,
}

/// Broadcast ssdp:alive messages on the local network's multicast SSDP channel on behalf of a remote DLNA server.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct CommandLineConf {

    /// TOML config file.
    #[clap(short, long, value_name = "/path/to/config.conf", conflicts_with_all(&["description-url", "interval", "proxy"]))] 
    config: Option<PathBuf>,

    /// URL pointing to the remote DLNA server's root XML description.
    #[clap(short = 'u', long, value_name = "URL", required_unless_present("config"), value_parser = Url::parse)]
    description_url: Option<Url>,

    /// Interval at which we will check the remote server's presence and broadcast on its behalf, in seconds.
    #[clap(short = 'd', long, value_name = "DURATION")]
    interval: Option<u64>,

    /// IP address & port where to bind proxy.
    #[clap(short = 'p', long, value_name = "IP:PORT", value_parser)]
    proxy: Option<SocketAddr>,

    /// Network interface on which to broadcast (requires root or CAP_NET_RAW capability).
    #[clap(short, long, value_name = "IFACE", )] 
    iface: Option<String>,

    /// Verbosity level. The more v, the more verbose.
    #[clap(short, long, action=ArgAction::Count)] 
    verbose: usize,
}

fn main() -> Result<()> {

    let args = CommandLineConf::parse();

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

fn get_config(args: CommandLineConf) -> Result<Config> {

    println!("{:?}", args);

    let config_as_file = args.config
        .map(|file| fs::read_to_string(file).map_err(|_| "Could not open/read config file."))
        .transpose()?;

    let (
            description_url,
            period,
            proxy,
            broadcast_iface,
            verbose

        ) = if let Some(config_file) = config_as_file {

            let raw_config: RawConfig = toml::from_str(&config_file).map_err(|e| {
                eprintln!("{}", e);
                "failed to parse config file."
            })?;

            let desc_url = raw_config.description_url
                .ok_or("Missing description URL")
                .and_then(|s| Url::parse(&s).map_err(|_| "Bad URL."))?;

            let period = raw_config.period;

            let proxy: Option<SocketAddr> = raw_config.proxy
                .as_deref()
                .map(str::parse)
                .transpose()
                .map_err(|_| "Bad address")?;

            (desc_url, period, proxy, raw_config.iface, raw_config.verbose)
        }
        else {
            (
                args.description_url.ok_or("Missing description URL")?,
                args.interval,
                args.proxy,
                args.iface,
                Some(args.verbose)
            )
        };

    let period = period.or(Some(895))
        .map(time::Duration::from_secs)
        .unwrap();

    let verbose = verbose
        .map_or(log::LevelFilter::Warn, |v| match v {
            0 => log::LevelFilter::Warn,
            1 => log::LevelFilter::Info,
            2 => log::LevelFilter::Debug,
            _ => log::LevelFilter::Trace,
    });

    Ok(Config {
        description_url, proxy, period, broadcast_iface, verbose
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
