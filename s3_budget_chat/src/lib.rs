use async_trait::async_trait;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use server::{Server, ServerErrorKind};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufStream};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};

/// Budget Chat
///
/// Modern messaging software uses too many computing resources,
/// so we're going back to basics. Budget Chat is a simple TCP-based chat room protocol.
///
/// Each message is a single line of ASCII text terminated by a newline character.
/// Clients can send multiple messages per connection. Servers may optionally strip
/// trailing whitespace, such as carriage return characters ('\r', or ASCII 13).
/// All messages are raw ASCII text, not wrapped up in JSON or any other format.
///
/// Upon connection
/// Setting the user's name
/// When a client connects to the server, it does not yet have a name and is not
/// considered to have joined. The server must prompt the user by sending a single
/// message asking for a name. The exact text of this message is implementation-defined.
///
/// Example:
///
/// `Welcome to budgetchat! What shall I call you?`
///
/// The first message from a client sets the user's name, which must contain at least 1 character,
/// and must consist entirely of alphanumeric characters (uppercase, lowercase, and digits).
///
/// Implementations may limit the maximum length of a name, but must allow at least 16 characters.
/// Implementations may choose to either allow or reject duplicate names.
///
/// If the user requests an illegal name, the server may send an informative error message to the client,
/// and the server must disconnect the client, without sending anything about the illegal user to any other clients.
///
/// Presence notification
///
/// Once the user has a name, they have joined the chat room and the server must announce their presence
/// to other users (see "A user joins" below).
///
/// In addition, the server must send the new user a message that lists all present users' names,
/// not including the new user, and not including any users who have already left.
/// The exact text of this message is implementation-defined, but must start with an asterisk ('*'),
/// and must contain the users' names. The server must send this message even if the room was empty.
///
/// Example:
///
/// `* The room contains: bob, charlie, dave`
///
/// All subsequent messages from the client are chat messages.
///
/// Chat messages
///
/// When a client sends a chat message to the server, the server must relay it to all other clients
/// as the concatenation of:
///
/// - open square bracket character
/// - the sender's name
/// - close square bracket character
/// - space character
/// - the sender's message
///
/// If "bob" sends "hello", other users would receive "[bob] hello".
///
/// Implementations may limit the maximum length of a chat message,
/// but must allow at least 1000 characters.
///
/// The server must not send the chat message back to the originating client,
/// or to connected clients that have not yet joined.
///
/// For example, if a user called "alice" sends a message saying "Hello, world!",
/// all users except alice would receive:
///
/// `[alice] Hello, world!`
///
/// A user joins
///
/// When a user joins the chat room by setting an acceptable name, the server must send
/// all other users a message to inform them that the user has joined.
/// The exact text of this message is implementation-defined, but must start with an asterisk ('*'),
/// and must contain the user's name.
///
/// Example:
///
/// `* bob has entered the room`
///
/// The server must send this message to other users that have already joined,
/// but not to connected clients that have not yet joined.
///
/// A user leaves
///
/// When a joined user is disconnected from the server for any reason, the server must
/// send all other users a message to inform them that the user has left.
/// The exact text of this message is implementation-defined, but must start with an asterisk ('*'),
/// and must contain the user's name.
///
/// Example:
///
/// `* bob has left the room`
///
/// The server must send this message to other users that have already joined,
/// but not to connected clients that have not yet joined.
///
/// If a client that has not yet joined disconnects from the server,
/// the server must not send any messages about that client to other clients.
///
/// Example session
///
/// In this example, "-->" denotes messages from the server to Alice's client,
/// and "<--" denotes messages from Alice's client to the server.
///
/// --> Welcome to budgetchat! What shall I call you?
/// <-- alice
/// --> * The room contains: bob, charlie, dave
/// <-- Hello everyone
/// --> [bob] hi alice
/// --> [charlie] hello alice
/// --> * dave has left the room
///
/// Alice connects and sets her name. She is given a list of users already in the room.
/// She sends a message saying "Hello everyone". Bob and Charlie reply. Dave disconnects.
///
/// Other requirements
///
/// Accept TCP connections.
///
/// Make sure you support at least 10 simultaneous clients.
#[derive(Debug, Default)]
pub struct BudgetChatServer {
    listener: Option<TcpListener>,
    state: Arc<Mutex<SharedState>>,
}

#[async_trait]
impl Server for BudgetChatServer {
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
                let (socket, addr) = l.accept().await.unwrap();

                println!("Connection open\n");

                // Clone the state to move into the new task.
                let state = self.state.clone();

                // A new task is spawned for each inbound socket. The socket is
                // moved to the new task and processed there.
                tokio::spawn(async move {
                    let mut client = ChatClient::new(addr, state);
                    client.process(socket).await
                });
            }
        } else {
            Err(ServerErrorKind::NotBound)
        }
    }
}

/// Custom Action type that is returned by the `Protocol`
///
/// This is used to define the action to be taken by the server
#[derive(Debug, PartialEq)]
enum ClientAction {
    Broadcast(Vec<u8>),
    Reply(String, Vec<u8>),
}

#[derive(Debug)]
enum ClientError {
    MalformedRequest(Vec<u8>),
    UnconformingName(Vec<u8>),
}

#[derive(Debug)]
struct ChatClient {
    addr: SocketAddr,
    name: Option<String>,
    state: ChatState,
    shared_state: Arc<Mutex<SharedState>>,
}

#[derive(Debug)]
enum ChatState {
    GetName,
    Chat,
}

impl ChatClient {
    fn new(addr: SocketAddr, state: Arc<Mutex<SharedState>>) -> Self {
        Self {
            addr,
            name: None,
            state: ChatState::GetName,
            shared_state: state,
        }
    }

    fn next_state(&mut self) {
        self.state = match self.state {
            ChatState::GetName => ChatState::Chat,
            ChatState::Chat => ChatState::Chat,
        }
    }

    /// method to process each received request/line
    async fn process_request(&mut self, line: &[u8]) -> Result<ClientAction, ClientError> {
        match self.state {
            ChatState::GetName => {
                let mut name = String::from_utf8(line.to_vec()).map_err(|_| {
                    ClientError::UnconformingName(b"Not a valid string for a name\n".to_vec())
                })?;

                name = name
                    .strip_suffix("\r\n")
                    .or(name.strip_suffix('\n'))
                    .unwrap_or(&name)
                    .to_string();

                if name.chars().all(|c| c.is_alphanumeric()) {
                    self.name = Some(name.clone());
                    self.next_state();

                    // Broadcast the name to all other peers
                    self.shared_state
                        .lock()
                        .await
                        .broadcast(
                            self.addr,
                            format!("* {} has entered the room\n", name).as_bytes(),
                        )
                        .await;

                    // Get the members list
                    let members = self
                        .shared_state
                        .lock()
                        .await
                        .peers
                        .values()
                        .map(|item| item.0.clone())
                        .collect::<Vec<String>>()
                        .join(",");

                    Ok(ClientAction::Reply(
                        name,
                        format!("* The room contains: {}\n", members)
                            .as_bytes()
                            .to_vec(),
                    ))
                } else {
                    Err(ClientError::UnconformingName(
                        b"Not a valid string for a name\n".to_vec(),
                    ))
                }
            }
            ChatState::Chat => {
                let mut msg = String::from_utf8(line.to_vec()).map_err(|_| {
                    ClientError::MalformedRequest(b"Not a valid string for a message\n".to_vec())
                })?;

                msg = msg.strip_suffix("\r\n")
                    .or(msg.strip_suffix('\n'))
                    .unwrap_or(&msg)
                    .to_string();

                Ok(ClientAction::Broadcast(
                    format!("[{}] {}\n", self.name.as_ref().unwrap(), msg)
                        .as_bytes()
                        .to_vec(),
                ))
            }
        }
    }

    /// Processes a connection
    ///
    /// Returns a `Result` which contains the number of processed bytes on the
    /// success path and a custom defined error `SolutionError`
    async fn process(&mut self, stream: TcpStream) -> Result<(), ServerErrorKind> {
        let mut stream = BufStream::new(stream);
        let mut line = vec![];
        let mut should_continue = true;

        // Get address of peer
        let addr = stream
            .get_ref()
            .peer_addr()
            .map_err(|_| ServerErrorKind::ReadFail)?;

        // Create a channel for the peer
        let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();

        // Send back the result
        stream
            .write_all(b"Welcome to budgetchat! What shall I call you?\n")
            .await
            .map_err(|_| ServerErrorKind::WriteFail)?;

        // Flush the buffer to ensure it is sent
        stream
            .flush()
            .await
            .map_err(|_| ServerErrorKind::WriteFail)?;

        let mut only_tx = Some(tx);

        // Loop until no bytes are read
        while should_continue {
            // Wait for messages from other peers or parse the received request
            tokio::select! {
                // If there's a message from another peer it is a chat message
                Some(msg) = rx.recv() => {
                    if !msg.is_empty() {
                        // Send back the result
                        stream
                            .write_all(&msg)
                            .await
                            .map_err(|_| ServerErrorKind::WriteFail)?;

                        // Flush the buffer to ensure it is sent
                        stream.flush().await.map_err(|_| ServerErrorKind::WriteFail)?;
                    }
                }
                // If there's a request incoming
                result = stream.read_until(b'\n', &mut line) => {
                    let read_len = result.map_err(|_| ServerErrorKind::ReadFail)?;

                    if read_len > 0 {
                        // Process the received request/line
                        let response = match self.process_request(&line).await {
                            Ok(ClientAction::Broadcast(arr)) => {
                                self.shared_state.lock().await.broadcast(addr, arr.as_slice()).await;
                                vec![]
                            }
                            // Only returned when a new member sets his name
                            Ok(ClientAction::Reply(name, arr)) => {
                                // Add the current peer to the list signaling it is up for receiving messages
                                if let Some(tx) = only_tx {
                                    self.shared_state.lock().await.peers.insert(self.addr, (name, tx));
                                    only_tx = None;
                                }
                                arr
                            }
                            Err(ClientError::MalformedRequest(arr)) | Err(ClientError::UnconformingName(arr)) => {
                                // In case an error occures stop reading
                                should_continue = false;
                                arr
                            }
                        };

                        // If there's something to send
                        if !response.is_empty() {
                            // Send back the result
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

                    line.clear();
                }
            }
        }

        // Remove peer from shared state list
        self.shared_state.lock().await.peers.remove(&self.addr);

        Ok(())
    }
}

/// Shorthand for the transmit half of the message channel.
type Tx = mpsc::UnboundedSender<Vec<u8>>;

#[derive(Debug, Default)]
struct SharedState {
    peers: HashMap<SocketAddr, (String, Tx)>,
}

impl SharedState {
    async fn broadcast(&mut self, from: SocketAddr, message: &[u8]) {
        for peer in self.peers.iter_mut() {
            if *peer.0 != from {
                let _ = peer.1 .1.send(message.into());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_request() {
        let mut solution = ChatClient;
        let line = b"hello world\n";
        let res = solution.process_request(line);
        assert_eq!(res.is_ok(), true);
        assert_eq!(res.unwrap(), line.to_vec());
    }
}
