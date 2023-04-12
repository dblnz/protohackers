use async_trait::async_trait;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use traits::Protocol;

/// Error type that is returned by the `SolutionServer`
#[derive(Debug)]
pub enum ServerErrorKind {
    BindFail,
    NotBound,
    ReadFail,
    WriteFail,
}


enum ServerState {
    Idle,
    Bound,
    Listening,
}

enum ConnectionState {
    Idle,
    Reading,
    Processing,
    Writing,
}

struct Connection<T> {
    solution: T,
    state: ConnectionState,
}

impl<T: Default + Protocol> Connection<T> {
    fn new() -> Self {
        Self {
            solution: T::default(),
            state: ConnectionState::Idle,
        }
    }
}

pub struct Server<T> {
    listener: Option<TcpListener>,
    conns: Arc<Vec<Connection<T>>>,
    state: Arc<ServerState>,
}

impl<T: Protocol> Default for Server<T> {
    fn default() -> Self {
        Self { listener: None,  state: Arc::new(ServerState::Idle), conns: Arc::new(vec![])}
    }
}

impl<T: Protocol> Server<T> {
    /// Method that binds a server to the address:port given
    async fn bind(&mut self, addr: &str) -> Result<(), ServerErrorKind> {
        self.listener = Some(TcpListener::bind(addr).await.map_err(|_| ServerErrorKind::BindFail)?);

        Ok(())
    }

    /// Method that puts the server in listening mode that
    /// takes in new connections, reads requests and responds
    /// to them accordingly
    async fn listen(&mut self) -> Result<(), ServerErrorKind> {
        if let Some(l) = self.listener.as_ref() {
            loop {
                println!("Waiting for connection ...");
        
                // The second item contains the IP and port of the new connection.
                let (socket, _) = l.accept().await.unwrap();
        
                println!("Connection open\n");
                // A new task is spawned for each inbound socket. The socket is
                // moved to the new task and processed there.
                tokio::spawn(async move {
                    self.process(socket)
                });
            }
        }
        else {
            Err(ServerErrorKind::NotBound)
        }
    }
}

/// TODO: Create a generic server that takes a custom solution and handles internally
/// the states and communication between clients

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_success() {
        assert!(true);
    }
}