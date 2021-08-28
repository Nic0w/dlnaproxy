use log::{info, trace, warn, debug};

use std::{
    net::{
     SocketAddr, TcpListener, TcpStream
    },
    sync::Arc,
    thread::{
        self,
        JoinHandle
    },
    io
};

//Adapted from https://github.com/hishboy/rust-tcp-proxy/

pub struct TCPProxy;
impl TCPProxy {

    pub fn start(self, to: SocketAddr, from: SocketAddr) -> JoinHandle<()> {

        let listener = TcpListener::bind(from).
            expect("Unable to bind proxy addr");

        info!(target: "dlnaproxy", "Proxing TCP connections from {} to {}.", from, to);

        thread::spawn(self.listen_loop(listener, to))
    }

    fn listen_loop(&self, listener: TcpListener, origin: SocketAddr) -> impl FnOnce() {
        move || {

            for incoming_stream in listener.incoming() {

                let proxied_stream = if let Ok(stream) = incoming_stream {
                    stream
                }
                else {
                    continue;
                };

                let peer_addr = proxied_stream.peer_addr().
                    unwrap();

                let conn_thread = TcpStream::connect(origin)
                .map(|to_stream| thread::spawn(move || handle_conn(proxied_stream, to_stream)));

                match conn_thread {
                    Ok(_) => { debug!(target: "dlnaproxy", "Successfully established a connection with client: {}", peer_addr); }
                    Err(err) => { warn!(target: "dlnaproxy", "Unable to establish a connection with client: {}", err); }
                }
            }
        }
    }
}


fn handle_conn(lhs_stream: TcpStream, rhs_stream: TcpStream) {

    let peer_addr = lhs_stream.peer_addr()
        .unwrap();

    let lhs_arc = Arc::new(lhs_stream);
    let rhs_arc = Arc::new(rhs_stream);

    let (mut lhs_tx, mut lhs_rx) = (lhs_arc.try_clone().unwrap(), lhs_arc.try_clone().unwrap());
    let (mut rhs_tx, mut rhs_rx) = (rhs_arc.try_clone().unwrap(), rhs_arc.try_clone().unwrap());

    let connections = vec![
        thread::spawn(move || io::copy(&mut lhs_tx, &mut rhs_rx).unwrap()),
        thread::spawn(move || io::copy(&mut rhs_tx, &mut lhs_rx).unwrap()),
    ];

    for t in connections {
        t.join().unwrap();
    }

    trace!(target: "dlnaproxy", "Closed connection with: {}", peer_addr);
}
