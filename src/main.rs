use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio::net::TcpListener;

const IP: Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);
const PORT: u16 = 8080;

mod s0_smoke_test;
use s0_smoke_test::process;

#[tokio::main]
async fn main() {
    let addr = SocketAddr::new(IpAddr::V4(IP), PORT);

    println!("Listening on {:?}:{:?}...", IP, PORT);

    // Bind the listener to the address
    let listener = TcpListener::bind(&addr).await.unwrap();

    loop {
        println!("Waiting for connection ...");

        // The second item contains the IP and port of the new connection.
        let (socket, _) = listener.accept().await.unwrap();

        println!("Connection open\n");
        // A new task is spawned for each inbound socket. The socket is
        // moved to the new task and processed there.
        let handle = tokio::spawn(async move {
            // Do some async work
            process(socket).await
        });

        // Wait for the job
        let out = handle.await.unwrap();
        println!("Processing result: {:?}", out);
        println!("Connection closed\n");
    }
}
