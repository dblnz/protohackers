#![deny(warnings)]

use cfg_if::cfg_if;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio::net::{TcpListener, TcpStream};

mod solution;
use solution::{ProtoHSolution, SolutionError};

cfg_if! {
    if #[cfg(feature = "s0")] {
        mod s0_smoke_test;
        use s0_smoke_test::SmokeTestSolution;
        type Solution = SmokeTestSolution;
    } else if #[cfg(feature = "s1")] {
        mod s1_prime_time;
        use s1_prime_time::PrimeTimeSolution;
        type Solution = PrimeTimeSolution;
    } else if #[cfg(feature = "s2")] {
        mod s2_means_to_an_end;
        use s2_means_to_an_end::MeansToAnEndSolution;
        type Solution = MeansToAnEndSolution;
    }
    else {
        compile_error!("Either feature \"s0\", \"s1\" or \"s2\" must be enabled for this app.");
    }
}

const IP: Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);
const PORT: u16 = 8080;

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
        tokio::spawn(async move {
            // Do some async work
            match process(socket).await {
                Ok(len) => {
                    println!("Processing successful. Got: {} bytes", len);
                }
                Err(SolutionError::Read) => {
                    println!("There was a Read Error involved in the processing of the request");
                }
                Err(SolutionError::Request(_)) => {
                    println!(
                        "There was a General Type Error involved in the processing of the request"
                    );
                }
                Err(SolutionError::Write) => {
                    println!("There was a Write Error involved in the processing of the request");
                }
            }
        });
    }
}

pub async fn process(stream: TcpStream) -> Result<usize, SolutionError> {
    let mut s = Solution::new();

    s.handle_stream(stream).await
}
