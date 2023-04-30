#![deny(warnings)]

use cfg_if::cfg_if;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use server::{Server, ServerErrorKind};

// define Solution dependent on the feature enabled
cfg_if! {
    if #[cfg(feature = "s0")] {
        use s0_smoke_test::SmokeTestServer;
        type ServerSol = SmokeTestServer;
    } else if #[cfg(feature = "s1")] {
        use s1_prime_time::PrimeTimeServer;
        type ServerSol = PrimeTimeServer;
    } else if #[cfg(feature = "s2")] {
        use s2_means_to_an_end::MeansToAnEndServer;
        type ServerSol = MeansToAnEndServer;
    } else if #[cfg(feature = "s3")] {
        use s3_budget_chat::BudgetChatServer;
        type ServerSol = BudgetChatServer;
    }
    else {
        // compile_error!("Either feature \"s0\", \"s1\" or \"s2\" must be enabled for this app.");
        use s3_budget_chat::BudgetChatServer;
        type ServerSol = BudgetChatServer;
    }
}

// Define the IP and PORT to bind to
const IP: Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);
const PORT: u16 = 8080;

#[tokio::main]
async fn main() -> Result<(), ServerErrorKind> {
    let addr = SocketAddr::new(IpAddr::V4(IP), PORT);

    let mut server = ServerSol::default();

    // Bind the server to the address:port
    server.bind(&addr.to_string()).await?;

    // Start listening for new connections
    server.listen().await
}
