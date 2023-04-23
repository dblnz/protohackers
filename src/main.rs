#![deny(warnings)]

use cfg_if::cfg_if;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use server::{Server, ServerErrorKind};

// define Solution dependent on the feature enabled
cfg_if! {
    if #[cfg(feature = "s0")] {
        use s0_smoke_test::SmokeTestSolution;
        type Solution = SmokeTestSolution;
    } else if #[cfg(feature = "s1")] {
        use s1_prime_time::PrimeTimeSolution;
        type Solution = PrimeTimeSolution;
    } else if #[cfg(feature = "s2")] {
        use s2_means_to_an_end::MeansToAnEndSolution;
        type Solution = MeansToAnEndSolution;
    }
    else {
        // compile_error!("Either feature \"s0\", \"s1\" or \"s2\" must be enabled for this app.");
        use s2_means_to_an_end::MeansToAnEndSolution;
        type Solution = MeansToAnEndSolution;
    }
}

// Define the IP and PORT to bind to
const IP: Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);
const PORT: u16 = 8080;

#[tokio::main]
async fn main() -> Result<(), ServerErrorKind> {
    let addr = SocketAddr::new(IpAddr::V4(IP), PORT);

    let mut server = Server::default();

    // Bind the server to the address:port
    server.bind(&addr.to_string()).await?;

    // Start listening for new connections
    server.listen::<Solution>().await
}
