
use crate::ThreadConfig;
use crate::ssdp_broadcast;

use std::sync::Arc;

use std::net::{ UdpSocket };
use std::collections::HashMap;

use httparse::{Request, EMPTY_HEADER};

use log::{info, trace, warn, debug};

/*
    SSDP RFC for reference: https://tools.ietf.org/html/draft-cai-ssdp-v1-03
*/

pub fn do_listen(config: Arc<ThreadConfig>) {

    let ssdp = config.ssdp_socket.try_clone().
        expect("Failed to clone socket.");

    let http_client = config.http_client.clone();

    loop {
        let mut buffer: [u8; 1024] = [0; 1024];

        let (bytes_read, src_addr) = ssdp.recv_from(&mut buffer).
            expect("failed to read!");

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

        let src_addr = src_addr.to_string();

        //We have a valid ssdp:discover request, although the rfc is soooooo vague it hurts.
        if ssdp_method == "M-SEARCH" && st_header.is_some() {
            if st_header.unwrap() == "urn:schemas-upnp-org:device:MediaServer:1" {
                info!(target: "dlnaproxy", "Responding to a M-SEARCH request for a MediaServer from {sender}.", sender=src_addr);

                if let Ok(cloned_socket) = ssdp.try_clone() {
                    if let Err(msg) = ssdp_broadcast::do_ssdp_alive(&http_client, cloned_socket, &config.description_url) {
                        warn!(target: "dlnaproxy", "Failed to broadcast while trying to respond to a M-SEARCH request: {}", msg);
                    }
                }
                else {
                    warn!(target: "dlnaproxy", "Failed to clone socket for broadcast while trying to respond to a M-SEARCH request.");
                }
            }
        }
    }
}

fn parse_ssdp(buffer: &[u8]) -> Result<(String, HashMap<String, String>), &'static str> {

    let mut headers = [EMPTY_HEADER; 16];
    let mut req = Request::new(&mut headers);

    req.parse(buffer).
        map_err(|_| "Failed to parse packet as SSDP.")?;

    let method = req.method.map(|s| String::from(s))
        .ok_or_else(|| "No method SSDP found.")?;

    let mut header_map: HashMap<String, String> = HashMap::with_capacity(headers.len());
    let mut i = 0;
    while !headers[i].name.is_empty() {
        let name = String::from(headers[i].name).to_uppercase();
        let value = String::from_utf8_lossy(headers[i].value);

        header_map.insert(name, value.to_string());
        i +=1;
    }

    Ok((method, header_map))
}
