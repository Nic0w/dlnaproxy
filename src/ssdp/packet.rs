use chrono::Utc;
use std::{
    fmt,
    net::{ToSocketAddrs, UdpSocket},
};

use anyhow::{Context, Result};

pub enum SSDPPacket {
    Alive {
        desc_url: String,
        server_ua: String,
        unique_device_name: String,
        device_type: String,
        cache_max_age: usize,
    },
    Ok {
        desc_url: String,
        server_ua: String,
        unique_device_name: String,
        device_type: String,
        cache_max_age: usize,
    },
    ByeBye {
        unique_device_name: String,
        device_type: String,
    },
}

impl SSDPPacket {
    pub fn send_to(&self, socket: &UdpSocket, dest: impl ToSocketAddrs) -> Result<()> {
        socket
            .send_to(self.to_string().as_bytes(), dest)
            .context("Failed to send SSDP packet on UDP socket")?;

        Ok(())
    }
}

impl fmt::Display for SSDPPacket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SSDPPacket::Alive {
                desc_url,
                server_ua,
                unique_device_name,
                device_type,
                cache_max_age,
            } => {
                write!(
                    f,
                    "\
NOTIFY * HTTP/1.1\r\n\
HOST:239.255.255.250:1900\r\n\
CACHE-CONTROL:max-age={cache_max_age}\r\n\
LOCATION:{location}\r\n\
SERVER: {server_ua}\r\n\
NT:{device_type}\r\n\
USN:{udn}::{device_type}\r\n\
NTS:ssdp:alive\r\n\
\r\n",
                    cache_max_age = cache_max_age,
                    location = desc_url,
                    server_ua = server_ua,
                    device_type = device_type,
                    udn = unique_device_name
                )
            }

            SSDPPacket::Ok {
                desc_url,
                server_ua,
                unique_device_name,
                device_type,
                cache_max_age,
            } => {
                let now = Utc::now().to_rfc2822().replace("+0000", "GMT");

                write!(
                    f,
                    "\
HTTP/1.1 200 OK\r\n\
CACHE-CONTROL:max-age={cache_max_age}\r\n\
DATE: {date}\r\n\
ST: {device_type}\r\n\
USN:{udn}::{device_type}\r\n\
EXT:\r\n\
SERVER: {server_ua}\r\n\
LOCATION:{location}\r\n\
Content-Length: 0\r\n\
\r\n",
                    cache_max_age = cache_max_age,
                    location = desc_url,
                    server_ua = server_ua,
                    device_type = device_type,
                    udn = unique_device_name,
                    date = now
                )
            }

            SSDPPacket::ByeBye {
                unique_device_name,
                device_type,
            } => {
                write!(
                    f,
                    "\
NOTIFY * HTTP/1.1\r\n\
HOST:239.255.255.250:1900\r\n\
NT:{device_type}\r\n\
USN:{udn}::{device_type}\r\n\
NTS:ssdp:byebye\r\n\
\r\n",
                    device_type = device_type,
                    udn = unique_device_name
                )
            }
        }
    }
}
