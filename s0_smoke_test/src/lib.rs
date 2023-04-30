use async_trait::async_trait;

use server::{Server, ServerErrorKind};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufStream};
use tokio::net::{TcpListener, TcpStream};

/// Smoke Test - TCP Echo Service
///
/// This Protocol implies replying to the requests with
/// the same data received
#[derive(Debug, Default)]
pub struct SmokeTestServer {
    listener: Option<TcpListener>,
}

#[async_trait]
impl Server for SmokeTestServer {
    /// Method that binds a server to the address:port given
    async fn bind(&mut self, addr: &str) -> Result<(), ServerErrorKind> {
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
    async fn listen(&mut self) -> Result<(), ServerErrorKind> {
        if let Some(l) = self.listener.as_ref() {
            loop {
                println!("Waiting for connection ...");

                // The second item contains the IP and port of the new connection.
                let (socket, _) = l.accept().await.unwrap();

                println!("Connection open\n");

                // A new task is spawned for each inbound socket. The socket is
                // moved to the new task and processed there.
                tokio::spawn(async move { process(socket).await });
            }
        } else {
            Err(ServerErrorKind::NotBound)
        }
    }
}

/// Processes a connection
///
/// Returns a `Result` which is empty on the success path and
/// contains a `ServerErrorKind` on the error path
async fn process(stream: TcpStream) -> Result<(), ServerErrorKind> {
    let mut stream = BufStream::new(stream);
    let mut line = vec![];
    let mut should_continue = true;

    while should_continue {
        let read_len = stream
            .read_until(b'\n', &mut line)
            .await
            .map_err(|_| ServerErrorKind::ReadFail)?;

        if read_len > 0 {
            // Process the received request/line
            let response = &line;

            // If there's something to send
            if !response.is_empty() {
                // Send back the result
                stream
                    .write_all(response)
                    .await
                    .map_err(|_| ServerErrorKind::WriteFail)?;

                // Flush the buffer to ensure it is sent
                stream
                    .flush()
                    .await
                    .map_err(|_| ServerErrorKind::WriteFail)?;
            }
        } else {
            should_continue = false;
        }

        line.clear();
    }

    Ok(())
}
