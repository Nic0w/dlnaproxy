mod config;
mod ssdp;
mod tcp_proxy;

use std::{net::SocketAddr, path::PathBuf, time};

use config::Config;

use reqwest::Url;

use anyhow::Result;
use clap::{ArgAction, Parser};
use log::{debug, trace};
use ssdp::main_task;

use crate::ssdp::SSDPManager;
use crate::tcp_proxy::TCPProxy;

/// Broadcast ssdp:alive messages on the local network's multicast SSDP channel on behalf of a remote DLNA server.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct CommandLineConf {
    /// TOML config file.
    #[clap(short, long, value_name = "/path/to/config.conf", conflicts_with_all(&["description_url", "interval", "proxy"]))]
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
    #[clap(short, long, value_name = "IFACE")]
    iface: Option<String>,

    /// Verbosity level. The more v, the more verbose.
    #[clap(short, long, action=ArgAction::Count)]
    verbose: u8,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = CommandLineConf::parse();

    let config = Config::try_from(args)?;

    init_logging(config.verbose);

    let mut url = config.description_url;

    let _tcp_proxy_thread = if let Some(proxy_addr) = config.proxy {
        let server_addr = config::sockaddr_from_url(&url);

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
    )
    .await?;

    let handle = tokio::spawn(main_task(ssdp));

    let _ = handle.await;

    Ok(())
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
