use std::net::{ UdpSocket, Ipv4Addr };

fn main() {
    println!("Hello, world!");

    let multicast_addr = Ipv4Addr::new(239, 255, 255, 250);
    let port: u16 = 1900;

    let bind_addr = format!("{addr}:{port}", addr=multicast_addr, port=port);

    let ssdp = UdpSocket::bind(bind_addr).
        expect("Failed to bind socket.");

    ssdp.join_multicast_v4(&multicast_addr, &Ipv4Addr::UNSPECIFIED).
        expect("Failed to join multicast group");

    let mut buffer: [u8; 1024] = [0; 1024];

    let (read, src) = ssdp.recv_from(&mut buffer).expect("failed to read!");

    println!("Read {} bytes from {}.", read, src);

}
