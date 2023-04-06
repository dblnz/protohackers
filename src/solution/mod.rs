use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufStream};

/// Custom Error type used to treat Solution specific errors
#[derive(Debug)]
pub enum SolutionError {
    InvalidRequest(Vec<u8>),
    InvalidRead,
    InvalidWrite,
}

#[async_trait]
pub trait ProtoHSolution {
    /// Custom method to process each received request/line
    fn process_request(&mut self, line: &[u8]) -> Result<Vec<u8>, SolutionError>;

    /// Handles a stream that produces requests the Solution has to respond to.
    ///
    /// Returns a `Result` which contains the number of processed bytes on the
    /// success path and a custom defined error `SolutionError`
    ///
    /// This method can be customly implemented for a new solution to produce
    /// requests in a different way(e.g. multi line)
    async fn handle_stream<T>(&mut self, socket: T) -> Result<usize, SolutionError>
    where
        T: AsyncReadExt + AsyncWriteExt + Send + Sync + Unpin,
    {
        let mut stream = BufStream::new(socket);
        let mut line = String::new();
        let mut len = 0;
        let mut should_continue = true;

        // Loop until no bytes are read
        while should_continue {
            // Read line - Return ReadError in case of fail
            let read_len = stream
                .read_line(&mut line)
                .await
                .map_err(|_| SolutionError::InvalidRead)?;

            if read_len > 0 {
                len += read_len;

                // Process the received request/line
                let response = match self.process_request(&line.as_bytes()) {
                    Ok(arr) => arr,
                    Err(SolutionError::InvalidRequest(arr)) => {
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

                // Send back the result
                stream
                    .write_all(&response)
                    .await
                    .map_err(|_| SolutionError::InvalidWrite)?;

                // Flush the buffer to ensure it is sent
                stream
                    .flush()
                    .await
                    .map_err(|_| SolutionError::InvalidWrite)?;
            } else {
                should_continue = false;
            }

            line.clear();
        }

        Ok(len)
    }
}
