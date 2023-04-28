use std::collections::HashMap;
use std::net::SocketAddr;
use std::marker::Sync;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufStream};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};

/// Custom Error type used to treat Solution specific errors
#[derive(Debug, PartialEq)]
pub enum SolutionError {
    MalformedRequest(Vec<u8>),
    Read,
    Write,
}

#[derive(Debug)]
pub enum RequestDelimiter {
    UntilChar(u8),
    NoOfBytes(usize),
}

pub trait Protocol
    where
        Self: Sync {
    /// Static method to get the delimiter between two requests
    /// This should be statically defined by each Custom solution
    ///
    /// The default implementation sets newline as the delimiter
    fn get_delimiter(&self) -> RequestDelimiter {
        // Return newline
        RequestDelimiter::UntilChar(b'\n')
    }

    /// Custom method to process each received request/line
    fn process_request(&mut self, line: &[u8]) -> Result<Action, SolutionError>;
}

/// Custom Action type that is returned by the `Protocol`
///
/// This is used to define the action to be taken by the server
#[derive(Debug, PartialEq)]
pub enum Action {
    Reply(Vec<u8>),
    Send(SocketAddr, Vec<u8>),
    Broadcast(Vec<u8>),
}

/// Error type that is returned by the `SolutionServer`
#[derive(Debug)]
pub enum ServerErrorKind {
    BindFail,
    NotBound,
    ReadFail,
    WriteFail,
}

pub struct Server {
    listener: Option<TcpListener>,
    state: Arc<Mutex<SharedState>>,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            listener: None,
            state: Arc::new(Mutex::new(SharedState::default())),
        }
    }
}

impl Server {
    /// Method that binds a server to the address:port given
    pub async fn bind(&mut self, addr: &str) -> Result<(), ServerErrorKind> {
        self.listener = Some(
            TcpListener::bind(addr)
                .await
                .map_err(|_| ServerErrorKind::BindFail)?,
        );

        println!("Listening on {:?}", addr);

        Ok(())
    }

    /// Method that puts the server in listening mode that
    /// takes in new connections, reads requests and responds
    /// to them accordingly
    pub async fn listen<T: Default + Protocol + Send + Sync>(
        &mut self,
    ) -> Result<(), ServerErrorKind> {
        if let Some(l) = self.listener.as_ref() {
            loop {
                println!("Waiting for connection ...");

                // The second item contains the IP and port of the new connection.
                let (socket, _) = l.accept().await.unwrap();

                println!("Connection open\n");

                let state = self.state.clone();

                // A new task is spawned for each inbound socket. The socket is
                // moved to the new task and processed there.
                tokio::spawn(async move {
                    let sol = T::default();
                    process(state, sol, socket).await
                });
            }
        } else {
            Err(ServerErrorKind::NotBound)
        }
    }
}

async fn read(stream: &mut BufStream<TcpStream>, line: &mut Vec<u8>, delimiter: RequestDelimiter) -> Result<usize, SolutionError> {
    let mut len = 0;
    let mut line = line;

    match delimiter {
        RequestDelimiter::UntilChar(del) => {
            len = stream.read_until(del, &mut line).await.map_err(|_| SolutionError::Read)?;
        }
        RequestDelimiter::NoOfBytes(n) => {
            *line = vec![0; n];
            len = stream.read_exact(&mut line).await.map_err(|_| SolutionError::Read)?;
        }
    }

    if len > 0 {
        Ok(len)
    } else {
        Err(SolutionError::Read)
    }
}

/// Processes a connection
///
/// Returns a `Result` which contains the number of processed bytes on the
/// success path and a custom defined error `SolutionError`
///
/// TODO: Modify -> This method can be customly implemented for a new solution to produce
/// requests in a different way(e.g. multi line)
async fn process<T: Default + Protocol + Send + Sync>(
    state: Arc<Mutex<SharedState>>,
    solution: T,
    stream: TcpStream,
) -> Result<(), SolutionError> {
    let mut solution = solution;
    let mut stream = BufStream::new(stream);
    let mut line = vec![];
    let mut should_continue = true;
    let mut _len = 0;

    // Get address of peer
    let addr = stream
        .get_ref()
        .peer_addr()
        .map_err(|_| SolutionError::Read)?;

    // Create a channel for the peer
    let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();

    // Add entry to this peer in the shared state
    state.lock().await.peers.insert(addr, tx);

    // Loop until no bytes are read
    while should_continue {
        // Wait for messages from other peers or parse the received request
        tokio::select! {
            Some(msg) = rx.recv() => {
                println!("rx got {:?}", msg);
    
                // If there's something to send
                if !msg.is_empty() {
                    // Send back the result
                    stream
                        .write_all(&msg)
                        .await
                        .map_err(|_| SolutionError::Write)?;
        
                    // Flush the buffer to ensure it is sent
                    stream.flush().await.map_err(|_| SolutionError::Write)?;
                }
            }
            result = read(&mut stream, &mut line, solution.get_delimiter()) => {
                println!("read from stream {:?}", result);
                // TODO: Handle error
                let read_len = result.unwrap();
                if read_len > 0 {
                    _len += read_len;
            
                    // Process the received request/line
                    let response = match solution.process_request(&line) {
                        Ok(Action::Reply(arr)) => arr,
                        Ok(Action::Send(addr, arr)) => {
                            unimplemented!()
                        }
                        Ok(Action::Broadcast(arr)) => {
                            state.lock().await.broadcast(addr, arr.as_slice()).await;
                            vec![]
                        }
                        Err(SolutionError::MalformedRequest(arr)) => {
                            // In case an error occures stop reading
                            should_continue = false;
                            arr
                        }
                        Err(_) => {
                            // In case an error occures stop reading
                            should_continue = false;
                            vec![]
                        }
                    };
            
                    // If there's something to send
                    if !response.is_empty() {
                        // Send back the result
                        stream
                            .write_all(&response)
                            .await
                            .map_err(|_| SolutionError::Write)?;
            
                        // Flush the buffer to ensure it is sent
                        stream.flush().await.map_err(|_| SolutionError::Write)?;
                    }
                } else {
                    should_continue = false;
                }
            
                line.clear();
            }
        }
    }

    Ok(())
}

/// Shorthand for the transmit half of the message channel.
type Tx = mpsc::UnboundedSender<Vec<u8>>;

struct SharedState {
    peers: HashMap<SocketAddr, Tx>,
}

impl Default for SharedState {
    fn default() -> Self {
        Self {
            peers: HashMap::new(),
        }
    }
}

impl SharedState {
    fn new() -> Self {
        Self::default()
    }

    async fn send(&mut self, _to: SocketAddr, _from: SocketAddr, _message: &[u8]) {
        unimplemented!()
    }

    async fn broadcast(&mut self, from: SocketAddr, message: &[u8]) {
        for peer in self.peers.iter_mut() {
            if *peer.0 != from {
                let _ = peer.1.send(message.into());
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_success() {
        assert!(true);
    }
}
