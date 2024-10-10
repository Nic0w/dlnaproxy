use anyhow::{anyhow, Context, Result};
use std::{
    fs,
    net::{SocketAddr, ToSocketAddrs as _},
    time,
};

use reqwest::Url;
use serde::Deserialize;

use crate::CommandLineConf;

#[derive(Deserialize)]
struct RawConfig {
    description_url: Option<String>,
    period: Option<u64>,
    proxy: Option<String>,
    verbose: Option<u8>,
    iface: Option<String>,
}

pub struct Config {
    pub description_url: Url,
    pub period: time::Duration,
    pub proxy: Option<SocketAddr>,
    pub broadcast_iface: Option<String>,
    pub verbose: log::LevelFilter,
}

impl TryFrom<CommandLineConf> for Config {
    type Error = anyhow::Error;

    fn try_from(conf: CommandLineConf) -> std::result::Result<Self, Self::Error> {
        get_config(conf)
    }
}

fn get_config(args: CommandLineConf) -> Result<Config> {
    println!("{:?}", args);

    let config_as_file = args
        .config
        .map(|file| fs::read_to_string(file).context("Could not open/read config file."))
        .transpose()?;

    let (description_url, period, proxy, broadcast_iface, verbose) =
        if let Some(config_file) = config_as_file {
            let raw_config: RawConfig =
                toml::from_str(&config_file).context("failed to parse config file.")?;

            let desc_url = raw_config
                .description_url
                .ok_or(anyhow!("Missing description URL"))
                .and_then(|s| Url::parse(&s).context("Bad description URL."))?;

            let period = raw_config.period;

            let proxy: Option<SocketAddr> = raw_config
                .proxy
                .as_deref()
                .map(str::parse)
                .transpose()
                .context("Bad proxy address")?;

            (
                desc_url,
                period,
                proxy,
                raw_config.iface,
                raw_config.verbose,
            )
        } else {
            (
                args.description_url
                    .ok_or(anyhow!("Missing description URL"))?,
                args.interval,
                args.proxy,
                args.iface,
                Some(args.verbose),
            )
        };

    let period = period.or(Some(895)).map(time::Duration::from_secs).unwrap();

    let verbose = verbose.map_or(log::LevelFilter::Warn, |v| match v {
        0 => log::LevelFilter::Warn,
        1 => log::LevelFilter::Info,
        2 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    });

    Ok(Config {
        description_url,
        proxy,
        period,
        broadcast_iface,
        verbose,
    })
}

pub fn sockaddr_from_url(url: &Url) -> SocketAddr {
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
