use log::debug;
use log::{info, trace, warn};

use std::borrow::Cow;
use std::{collections::HashMap, sync::Arc};
use tokio::net::UdpSocket;

use httparse::{Request, EMPTY_HEADER};

use anyhow::Context;
use anyhow::Result;

use crate::ssdp::utils::InteractiveSSDP;

/*
    SSDP RFC for reference: https://tools.ietf.org/html/draft-cai-ssdp-v1-03
*/

fn parse_ssdp(buffer: &[u8]) -> Result<(String, HashMap<String, Cow<'_, str>>)> {
    let mut headers = [EMPTY_HEADER; 16];
    let mut req = Request::new(&mut headers);

    req.parse(buffer)
        .context("Failed to parse packet as SSDP.")?;

    let method = req
        .method
        .map(String::from)
        .ok_or(super::error::Error::NoSSDPMethod)?;

    let mut header_map: HashMap<String, Cow<'_, str>> = HashMap::with_capacity(headers.len());
    let mut i = 0;
    while !headers[i].name.is_empty() {
        let name = String::from(headers[i].name).to_uppercase();
        let value = String::from_utf8_lossy(headers[i].value);

        header_map.insert(name, value);
        i += 1;
    }

    Ok((method, header_map))
}

pub async fn listen_task(ssdp_socket: Arc<UdpSocket>, ssdp_helper: Arc<InteractiveSSDP>) {
    debug!(target: "dlnaproxy", "Listen task up and running!");

    loop {
        let mut buffer: [u8; 1024] = [0; 1024];

        let (bytes_read, src_addr) = ssdp_socket
            .recv_from(&mut buffer)
            .await
            .expect("failed to read!");

        trace!(target: "dlnaproxy", "Read {amount} bytes sent by {sender}.", amount=bytes_read, sender=src_addr);

        let (ssdp_method, ssdp_headers) = match parse_ssdp(&buffer) {
            Ok(parsed_data) => parsed_data,
            Err(e) => {
                warn!(target:"dlnaproxy", "{}", e);
                continue;
            }
        };

        let st_header = ssdp_headers.get("ST");
        let _man_header = ssdp_headers.get("MAN");

        //We have a valid ssdp:discover request, although the rfc is soooooo vague it hurts.
        if let Some(header) = st_header {
            if ssdp_method == "M-SEARCH" && header == "urn:schemas-upnp-org:device:MediaServer:1" {
                info!(target: "dlnaproxy", "Responding to a M-SEARCH request for a MediaServer from {sender}.", sender=src_addr);

                if let Err(msg) = ssdp_helper.send_ok(&ssdp_socket, src_addr).await {
                    warn!(target: "dlnaproxy", "Couldn't send ssdp:alive: {}", msg);
                } else {
                    info!(target: "dlnaproxy", "Sent ssdp:ok on local SSDP channel!");
                }
            }
        }
    }
}
