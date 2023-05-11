use async_trait::async_trait;

use server::{Server, ServerErrorKind};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufStream};
use tokio::net::{TcpListener, TcpStream};

/// Mob in the Middle
///
/// You're escorted to a dark, smoky, basement office.
/// Big Tony sits the other side of a large desk, leaning back in his chair,
///  puffing on a cigar that you can only describe as comedically-oversized.
/// Two of his goons loiter in the doorway. They are tall and wide but not
///  obviously very bright, which only makes them all the more intimidating.
/// Tony flashes a menacing grin, revealing an unusual number of gold-plated teeth,
///  and makes you an offer you can't refuse: he wants you to write a malicious
///  proxy server for Budget Chat.
///
/// For each client that connects to your proxy server, you'll make a corresponding
///  outward connection to the upstream server. When the client sends a message to
///  your proxy, you'll pass it on upstream. When the upstream server sends a message
///  to your proxy, you'll pass it on downstream. Remember that messages in Budget Chat
///  are delimited by newline characters ('\n', or ASCII 10).
///
/// Most messages are passed back and forth without modification, so that the client
///  believes it is talking directly to the upstream server, except that you will be
///  rewriting Boguscoin addresses, in both directions, so that all payments go to Tony.
///
/// Connecting to the upstream server
///
/// The upstream Budget Chat server is at chat.protohackers.com on port 16963.
/// You can connect using either IPv4 or IPv6.
///
/// Rewriting Boguscoin addresses
///
/// Tony is trying to steal people's cryptocurrency. He has already arranged to have
///  his victim's internet connections compromised, and to have their Budget Chat
///  sessions re-routed to your proxy server.
///
/// Your server will rewrite Boguscoin addresses, in both directions,
///  so that they are always changed to Tony's address instead.
///
/// A substring is considered to be a Boguscoin address if it satisfies all of:
/// - it starts with a "7"
/// - it consists of at least 26, and at most 35, alphanumeric characters
/// - it starts at the start of a chat message, or is preceded by a space
/// - it ends at the end of a chat message, or is followed by a space
///
/// You should rewrite all Boguscoin addresses to Tony's address, which is 7YWHMfk9JZe0LM0g1ZauHuiSxhI.
///
/// Some more example Boguscoin addresses:
/// - 7F1u3wSD5RbOHQmupo9nx4TnhQ
/// - 7iKDZEwPZSqIvDnHvVN2r0hUWXD5rHX
/// - 7LOrwbDlS8NujgjddyogWgIM93MV5N2VR
/// - 7adNeSwJkMakpEcln9HEtthSRtxdmEHOT8T
///
/// Example session
///
/// In this first example, "-->" denotes messages from the proxy server to Bob's client, and "<--"
///  denotes messages from Bob's client to the proxy server.
///
/// --> Welcome to budgetchat! What shall I call you?
/// <-- bob
/// --> * The room contains: alice
/// <-- Hi alice, please send payment to 7iKDZEwPZSqIvDnHvVN2r0hUWXD5rHX
///
/// Bob connects to the server and asks Alice to send payment.
///
/// In this next example, "-->" denotes messages from the upstream server to the proxy server,
///  and "<--" denotes messages from the proxy server to the upstream server.
///
/// --> Welcome to budgetchat! What shall I call you?
/// <-- bob
/// --> * The room contains: alice
/// <-- Hi alice, please send payment to 7YWHMfk9JZe0LM0g1ZauHuiSxhI
///
/// Bob's Boguscoin address has been replaced with Tony's, but everything else is unchanged.
/// If Alice sends payment to this address, it will go to Tony instead of Bob.
/// Tony will be pleased, and will elect not to have his goons break your kneecaps.
///
/// Other requirements
///
/// Make sure your proxy server supports at least 10 simultaneous clients.
///
/// When either a client or an upstream connection disconnects from your proxy server,
///  disconnect the other side of the same session. (But you don't have to worry about
///  half-duplex shutdowns.)
///
/// As a reminder, Tony's Boguscoin address is:
///
/// 7YWHMfk9JZe0LM0g1ZauHuiSxhI
#[derive(Debug, Default)]
pub struct MobInTheMiddleServer;

#[async_trait]
impl Server for MobInTheMiddleServer {
    /// Run the server
    async fn run(&mut self, addr: &str) -> Result<(), ServerErrorKind> {
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|_| ServerErrorKind::BindFail)?;

        println!("Listening on {:?}", addr);

        loop {
            println!("Waiting for connection ...");

            // The second item contains the IP and port of the new connection.
            let (stream, _) = listener.accept().await.unwrap();

            println!("Connection open\n");

            // A new task is spawned for each inbound socket. The socket is
            // moved to the new task and processed there.
            tokio::spawn(async move {
                let mut client = ProxyClient::new();

                client.run(stream).await
            });
        }
    }
}

#[derive(Debug)]
struct ProxyClient;

impl ProxyClient {
    fn new() -> Self {
        Self {}
    }

    /// Runs a proxy client
    async fn run(&mut self, stream: TcpStream) -> Result<(), ServerErrorKind> {
        let mut stream = BufStream::new(stream);
        let mut line = vec![];
        let mut upstream_line = vec![];
        let mut should_continue = true;

        // Connect to the upstream server
        let upstream = TcpStream::connect("chat.protohackers.com:16963")
            .await
            .map_err(|_| ServerErrorKind::ConnectFail)?;

        // Create a buffered stream for the upstream connection
        let mut upstream = BufStream::new(upstream);

        // Loop until no bytes are read
        while should_continue {
            // Wait for messages from either the client or the upstream server
            tokio::select! {
                // Read a line from the client
                result = stream.read_until(b'\n', &mut line) => {
                    let read_len = result.map_err(|_| ServerErrorKind::ReadFail)?;

                    if read_len > 0 {
                        // Process the received request/line and send it to the upstream server
                        let response = self.process_request(&line);

                        // If there's something to send
                        if !response.is_empty() {
                            upstream
                                .write_all(&response)
                                .await
                                .map_err(|_| ServerErrorKind::WriteFail)?;

                            // Flush the buffer to ensure it is sent
                            upstream.flush().await.map_err(|_| ServerErrorKind::WriteFail)?;
                        }
                    } else {
                        should_continue = false;
                    }

                    line.clear();
                }
                // Read a line from the upstream server
                result = upstream.read_until(b'\n', &mut upstream_line) => {
                    let read_len = result.map_err(|_| ServerErrorKind::ReadFail)?;

                    if read_len > 0 {
                        // Process the received request/line and send it to the client
                        let response = self.process_request(&upstream_line);

                        // If there's something to send
                        if !response.is_empty() {
                            stream
                                .write_all(&response)
                                .await
                                .map_err(|_| ServerErrorKind::WriteFail)?;

                            // Flush the buffer to ensure it is sent
                            stream.flush().await.map_err(|_| ServerErrorKind::WriteFail)?;
                        }
                    } else {
                        should_continue = false;
                    }

                    upstream_line.clear();
                }
            }
        }

        Ok(())
    }

    /// Processes a request
    fn process_request(&mut self, request: &[u8]) -> Vec<u8> {
        // Check if the request is a Boguscoin address
        let mut request = request.to_vec();

        for address in self.get_boguscoin_address(&request) {
            // If it is, replace it with Tony's address
            request = self.replace_boguscoin_address(&request, address);
        }

        request
    }

    /// Extracts a Boguscoin address from a request
    fn get_boguscoin_address(&self, request: &[u8]) -> Vec<Vec<u8>> {
        // Convert the request to a string
        let request = String::from_utf8_lossy(request);

        // Check if the request is a Boguscoin address
        request
            .split_whitespace()
            .filter(|word| word.starts_with('7') && word.len() >= 26 && word.len() <= 35)
            .map(|address| address.as_bytes().to_vec())
            .collect()
    }

    /// Replaces a Boguscoin address with Tony's address
    fn replace_boguscoin_address(&mut self, request: &[u8], address: Vec<u8>) -> Vec<u8> {
        // Convert the request to a string
        let request = String::from_utf8_lossy(request);

        // Convert the address to a string
        let address = String::from_utf8_lossy(&address);

        // Replace the Boguscoin address with Tony's address
        let response = request.replace(&*address, "7YWHMfk9JZe0LM0g1ZauHuiSxhI");

        // Return the response as bytes
        response.as_bytes().to_vec()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_boguscoin_address_success() {
        let proxy_client = ProxyClient::new();

        let request = b"Hi alice, please send payment to 7iKDZEwPZSqIvDnHvVN2r0hUWXD5rHX";

        let address = proxy_client.get_boguscoin_address(request);

        assert_eq!(address, vec![b"7iKDZEwPZSqIvDnHvVN2r0hUWXD5rHX"]);
    }

    #[test]
    fn test_get_boguscoin_address_only_boguscoin_address() {
        let proxy_client = ProxyClient::new();

        let request = b"7iKDZEwPZSqIvDnHvVN2r0hUWXD5rHX";

        let address = proxy_client.get_boguscoin_address(request);

        assert_eq!(address, vec![b"7iKDZEwPZSqIvDnHvVN2r0hUWXD5rHX"]);
    }

    #[test]
    fn test_get_boguscoin_address_only_boguscoin_and_spaces_success() {
        let proxy_client = ProxyClient::new();

        let request = b" 7iKDZEwPZSqIvDnHvVN2r0hUWXD5rHX ";

        let address = proxy_client.get_boguscoin_address(request);

        assert_eq!(address, vec![b"7iKDZEwPZSqIvDnHvVN2r0hUWXD5rHX"]);
    }

    #[test]
    fn test_get_boguscoin_address_smaller_length_fail() {
        let proxy_client = ProxyClient::new();

        let request = b"Hi alice, please send payment to 7iKDZEwPZSqIvDnHvVN2r0hUW";

        let address = proxy_client.get_boguscoin_address(request);

        assert!(address.is_empty());
    }

    #[test]
    fn test_get_boguscoin_address_bigger_length_fail() {
        let proxy_client = ProxyClient::new();

        let request = b"Hi alice, please send payment to 7iKDZEwPZSqIvDnHvVN2r0hUWXD5rHXXADbc";

        let address = proxy_client.get_boguscoin_address(request);

        assert!(address.is_empty());
    }

    #[test]
    fn test_get_multiple_boguscoin_addresses_success() {
        let proxy_client = ProxyClient::new();

        let request = b"Hi alice, please send payment to 7iKDZEwPZSqIvDnHvVN2r0hUWXD5rHX or 7iKDZEwPZSqIvDnHvVN2r0hUWXD5rHY";

        let address = proxy_client.get_boguscoin_address(request);

        assert_eq!(
            address,
            vec![
                b"7iKDZEwPZSqIvDnHvVN2r0hUWXD5rHX",
                b"7iKDZEwPZSqIvDnHvVN2r0hUWXD5rHY"
            ]
        );
    }

    #[test]
    fn test_process_request_same_address_three_times_success() {
        let mut proxy_client = ProxyClient::new();

        let request = b"Please pay the ticket price of 15 Boguscoins to one of these addresses: 7pKIoehdvl0rDwzM5Alx5LadEG2M 7pKIoehdvl0rDwzM5AlP5LZEG2M 7gsqcHYJ2HCvxgOlVWN4YTDHNybbDpYoZ";

        let response = proxy_client.process_request(request);

        assert_eq!(
            String::from_utf8(response).unwrap(),
            "Please pay the ticket price of 15 Boguscoins to one of these addresses: 7YWHMfk9JZe0LM0g1ZauHuiSxhI 7YWHMfk9JZe0LM0g1ZauHuiSxhI 7YWHMfk9JZe0LM0g1ZauHuiSxhI".to_string()
        );
    }
}
