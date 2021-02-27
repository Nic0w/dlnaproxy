use std::net::{ UdpSocket, Ipv4Addr };
use std::collections::HashMap;

use httparse::{Request, EMPTY_HEADER};

/*
    SSDP RFC for reference: https://tools.ietf.org/html/draft-cai-ssdp-v1-03
*/

pub fn ssdp_loop() {

    let multicast_addr = Ipv4Addr::new(239, 255, 255, 250);
    let port: u16 = 1900;

    let bind_addr = format!("{addr}:{port}", addr=multicast_addr, port=port);

    let ssdp = UdpSocket::bind(bind_addr).
        expect("Failed to bind socket.");

    ssdp.join_multicast_v4(&multicast_addr, &Ipv4Addr::UNSPECIFIED).
        expect("Failed to join multicast group");

    loop {

        let mut buffer: [u8; 1024] = [0; 1024];

        let (_bytes_read, src_addr) = ssdp.recv_from(&mut buffer).
            expect("failed to read!");

        let (ssdp_method, ssdp_headers) = match parse_ssdp(&buffer) {
            Ok(parsed_data) => parsed_data,
            Err(e) => {
                println!("Error: {}", e);
                continue;
            }
        };

        let st_header = ssdp_headers.get("ST");
        let man_header = ssdp_headers.get("MAN");

        let src_addr = src_addr.to_string();

        let def_string = String::from("nothing");

        println!("{}:{} from {}, looking for {}", ssdp_method, man_header.or(Some(&def_string)).unwrap(), src_addr, st_header.or(Some(&def_string)).unwrap());

        //We have a valid ssdp:discover request, although the rfc is soooooo vague it hurts.
        if ssdp_method == "M-SEARCH" && st_header.is_some() {
            if st_header.unwrap() == "urn:schemas-upnp-org:device:MediaServer:1" {

                println!("{from} is searching for a MediaServer ! ", from=src_addr);

            }
        }
    }
}

fn parse_ssdp(buffer: &[u8]) -> Result<(String, HashMap<String, String>), &'static str> {

    let mut headers = [EMPTY_HEADER; 16];
    let mut req = Request::new(&mut headers);

    req.parse(buffer).
        map_err(|_| "failed to parse packet as SSDP.")?;

    let method = req.method.map(|s| String::from(s))
        .ok_or_else(|| "No method found.")?;

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
