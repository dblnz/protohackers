use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufStream};

/// Custom Error type used to treat Solution specific errors
#[derive(Debug, PartialEq)]
pub enum SolutionError {
    Request(Vec<u8>),
    Read,
    Write,
}

#[derive(Debug)]
pub enum RequestDelimiter {
    UntilChar(u8),
    NoOfBytes(usize),
}

#[async_trait]
pub trait Protocol {
    /// Static method to get the delimiter between two requests
    /// This should be statically defined by each Custom solution
    ///
    /// The default implementation sets newline as the delimiter
    fn get_delimiter() -> RequestDelimiter {
        // Return newline
        RequestDelimiter::UntilChar(b'\n')
    }

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
        let mut line = vec![];
        let mut len = 0;
        let mut should_continue = true;

        // Loop until no bytes are read
        while should_continue {
            // Read line - Return ReadError in case of fail
            let read_len = match Self::get_delimiter() {
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

                // Process the received request/line
                let response = match self.process_request(&line) {
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
