#![deny(warnings)]

use cfg_if::cfg_if;

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
    } else if #[cfg(feature = "s4")] {
        use s4_unusual_database_program::UnusualDatabaseProgramServer;
        type ServerSol = UnusualDatabaseProgramServer;
    } else if #[cfg(feature = "s5")] {
        use s5_mob_in_the_middle::MobInTheMiddleServer;
        type ServerSol = MobInTheMiddleServer;
    } else if #[cfg(feature = "s6")] {
        use s6_speed_daemon::SpeedDaemonServer;
        type ServerSol = SpeedDaemonServer;
    }
    else {
        use s6_speed_daemon::SpeedDaemonServer;
        type ServerSol = SpeedDaemonServer;
    }
}


#[tokio::main]
async fn main() -> Result<(), ServerErrorKind> {
    // define the address to bind to
    cfg_if! {
        // Use for UDP server
        if #[cfg(feature = "s4")] {
            let addr = "fly-global-services:8080".to_string();
        }
        // Use for TCP server
        else {
            use std::net::{IpAddr, Ipv4Addr, SocketAddr};

            // Define the IP and PORT to bind to
            const IP: Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);
            const PORT: u16 = 8080;

            let addr = SocketAddr::new(IpAddr::V4(IP), PORT);
        }
    }

    let mut server = ServerSol::default();

    // start the server on the address:port
    server.run(&addr.to_string()).await
}
