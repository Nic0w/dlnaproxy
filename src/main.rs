extern crate httparse;

use std::net::{ UdpSocket, Ipv4Addr };

use httparse::{Request, EMPTY_HEADER};

fn main() {
    println!("Hello, world!");

    let multicast_addr = Ipv4Addr::new(239, 255, 255, 250);
    let port: u16 = 1900;

    let bind_addr = format!("{addr}:{port}", addr=Ipv4Addr::UNSPECIFIED, port=port);

    let ssdp = UdpSocket::bind(bind_addr).
        expect("Failed to bind socket.");

    ssdp.join_multicast_v4(&multicast_addr, &Ipv4Addr::UNSPECIFIED).
        expect("Failed to join multicast group");

    let mut buffer: [u8; 1024] = [0; 1024];

    let (read, src) = ssdp.recv_from(&mut buffer).expect("failed to read!");

    println!("Read {} bytes from {}.", read, src);

    let data = String::from_utf8_lossy(&buffer);

    println!("{}", data);

    let mut headers = [EMPTY_HEADER; 10];
    let mut req = Request::new(&mut headers);

    req.parse(&buffer).expect("Parsing failed.");

    println!("Received {method} message !", method=req.method.unwrap());

    for i in 0..headers.len() {

        let header = headers[i];

        if header.name != "" {

            println!("{name}: {value}", name=header.name, value=String::from_utf8_lossy(header.value));
        }
    }

}
