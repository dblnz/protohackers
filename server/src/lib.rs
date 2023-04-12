use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufStream};
use traits::{Protocol, RequestDelimiter, SolutionError};

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

/// Struct used for handling a new incoming connection
struct Connection<T> {
    solution: T,
    state: ConnectionState,
    stream: TcpStream,
}

impl<T: Default + Protocol> Connection<T> {
    fn new(stream: TcpStream) -> Self {
        Self {
            solution: T::default(),
            state: ConnectionState::Idle,
            stream,
        }
    }

    /// Handles a stream that produces requests the Solution has to respond to.
    ///
    /// Returns a `Result` which contains the number of processed bytes on the
    /// success path and a custom defined error `SolutionError`
    ///
    /// This method can be customly implemented for a new solution to produce
    /// requests in a different way(e.g. multi line)
    async fn handle_stream<Q>(&mut self, socket: Q) -> Result<usize, SolutionError>
    where
        Q: AsyncReadExt + AsyncWriteExt + Send + Sync + Unpin,
    {
        let mut stream = BufStream::new(socket);
        let mut line = vec![];
        let mut len = 0;
        let mut should_continue = true;

        // Loop until no bytes are read
        while should_continue {
            self.state = ConnectionState::Reading;
            
            // Read line - Return ReadError in case of fail
            let read_len = match self.solution.get_delimiter() {
                RequestDelimiter::UntilChar(del) => stream
                    .read_until(del, &mut line)
                    .await
                    .map_err(|_| SolutionError::Read)?,
                RequestDelimiter::NoOfBytes(n) => {
                    line = vec![0; n];
                    stream
                        .read_exact(&mut line)
                        .await
                        .map_err(|_| SolutionError::Read)?
                }
            };

            if read_len > 0 {
                len += read_len;
                self.state = ConnectionState::Processing;

                // Process the received request/line
                let response = match self.solution.process_request(&line) {
                    Ok(arr) => arr,
                    Err(SolutionError::Request(arr)) => {
                        // In case an error occures stop reading
                        should_continue = false;
                        arr
                    }
                    _ => {
                        // In case an error occures stop reading
                        should_continue = false;
                        vec![]
                    }
                };

                // If there's something to send
                if !response.is_empty() {
                    self.state = ConnectionState::Writing;

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

        Ok(len)
    }
}

pub struct Server<T> {
    listener: Option<TcpListener>,
    conns: Vec<Arc<Mutex<Connection<T>>>>,
    state: Arc<ServerState>,
}

impl<T: Default + Protocol> Default for Server<T> {
    fn default() -> Self {
        Self { listener: None,  state: Arc::new(ServerState::Idle), conns: vec![]}
    }
}

impl<T: Default + Protocol + Send> Server<T> {
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
                let conn = Arc::new(Mutex::new(Connection::new(socket)));
                self.conns.push(conn.clone());

                // A new task is spawned for each inbound socket. The socket is
                // moved to the new task and processed there.
                tokio::spawn(async move {
                    process(conn)
                });
            }
        }
        else {
            Err(ServerErrorKind::NotBound)
        }
    }
}

async fn process<T: Default + Protocol + Send> (conn: Arc<Mutex<Connection<T>>>) -> Result<(), SolutionError> {
    let conn = conn.lock().unwrap();

    Ok(())
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