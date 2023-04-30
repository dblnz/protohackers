use async_trait::async_trait;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use server::{Server, ServerErrorKind};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufStream};
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
pub struct BudgetChatSolution {
    name: Option<String>,
    members: Vec<String>,
    state: ChatState,
}

#[derive(Debug, Default)]
enum ChatState {
    #[default]
    Joined,
    Sync,
    Welcome,
    BroadcastName,
    ShowMembers,
    Chat,
}

impl BudgetChatSolution {
    pub fn new() -> Self {
        Self {
            name: None,
            members: vec![],
            state: ChatState::Joined,
        }
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = Some(name.to_string());
    }

    pub fn add_member(&mut self, name: &str) {
        self.members.push(name.to_string());
    }

    pub fn remove_member(&mut self, name: &str) {
        self.members.retain(|n| n != name);
    }

    pub fn next_state(&mut self) {
        self.state = match self.state {
            ChatState::Joined => ChatState::Sync,
            ChatState::Sync => ChatState::Welcome,
            ChatState::Welcome => ChatState::BroadcastName,
            ChatState::BroadcastName => ChatState::ShowMembers,
            ChatState::ShowMembers => ChatState::Chat,
            ChatState::Chat => ChatState::Chat,
        }
    }

    /// method to get the delimiter between two requests
    fn get_delimiter(&self) -> RequestDelimiter {
        RequestDelimiter::UntilChar(b'\n')
    }

    /// method to get the welcome message
    fn welcome(&self) -> Option<Vec<u8>> {
        Some(b"Welcome to budgetchat! What shall I call you?\n".to_vec())
    }

    /// method to process each received request/line
    fn process_request(&mut self, line: &[u8]) -> Result<Action, SolutionError> {
        match self.state {
            ChatState::Joined => {
                // Advance state
                self.next_state();
                Ok(Action::Reply(
                    b"Welcome to budgetchat! What shall I call you?\n".to_vec(),
                ))
            }
            ChatState::Sync => {
                // Advance state
                self.next_state();
                Ok(Action::Reply(
                    b"* The room contains: bob, charlie, dave\n".to_vec(),
                ))
            }
            ChatState::Welcome => {
                // Advance state
                self.next_state();
                // Set name
                let name = std::str::from_utf8(line).unwrap().trim();
                self.set_name(name.clone());
                // Add member
                self.add_member(name);
                Ok(Action::Reply(
                    b"* The room contains: bob, charlie, dave\n".to_vec(),
                ))
            }
            ChatState::BroadcastName => {
                // Advance state
                self.next_state();
                // Broadcast name
                Ok(Action::Broadcast(
                    format!("* {} has entered the room\n", self.name.as_ref().unwrap())
                        .into_bytes(),
                ))
            }
            ChatState::ShowMembers => {
                // Advance state
                self.next_state();
                // Show members
                Ok(Action::Reply(
                    format!("* The room contains: {}\n", self.members.join(", ")).into_bytes(),
                ))
            }
            ChatState::Chat => {
                // Advance state
                self.next_state();
                // Broadcast message
                Ok(Action::Broadcast(
                    format!(
                        "[{}] {}\n",
                        self.name.as_ref().unwrap(),
                        std::str::from_utf8(line).unwrap().trim()
                    )
                    .into_bytes(),
                ))
            }
        }
    }

    /// method to process each received peer message
    fn process_peer_msg(&mut self, line: &[u8]) -> Result<Action, SolutionError> {
        unimplemented!()
    }
}

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

/// Shorthand for the transmit half of the message channel.
type Tx = mpsc::UnboundedSender<Vec<u8>>;

pub struct SharedState {
    peers: HashMap<SocketAddr, (Option<String>, Tx)>,
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
                let _ = peer.1 .1.send(message.into());
            }
        }
    }
}

pub struct BudgetChatServer {
    listener: Option<TcpListener>,
    state: Arc<Mutex<SharedState>>,
}

impl Default for BudgetChatServer {
    fn default() -> Self {
        Self {
            listener: None,
            state: Arc::new(Mutex::new(SharedState::default())),
        }
    }
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
                let (socket, _) = l.accept().await.unwrap();

                println!("Connection open\n");

                let state = self.state.clone();

                // A new task is spawned for each inbound socket. The socket is
                // moved to the new task and processed there.
                tokio::spawn(async move {
                    let sol = BudgetChatSolution::default();
                    process(state, sol, socket).await
                });
            }
        } else {
            Err(ServerErrorKind::NotBound)
        }
    }
}

/// This is needed to avoid the fact that the types of
/// `read_until` and `read_exact` are different
/// So we need to have a common function that can be used
async fn read_until_del(
    stream: &mut BufStream<TcpStream>,
    line: &mut Vec<u8>,
    delimiter: RequestDelimiter,
) -> Result<usize, SolutionError> {
    let mut line = line;

    let len = match delimiter {
        RequestDelimiter::UntilChar(del) => stream
            .read_until(del, &mut line)
            .await
            .map_err(|_| SolutionError::Read)?,
        RequestDelimiter::NoOfBytes(n) => {
            *line = vec![0; n];
            stream
                .read_exact(&mut line)
                .await
                .map_err(|_| SolutionError::Read)?
        }
    };

    if len > 0 {
        Ok(len)
    } else {
        Err(SolutionError::Read)
    }
}

/// This function welcomes a new client
///
/// This is solution specific, it might not send a
/// message after all
async fn welcome_client(
    solution: &mut BudgetChatSolution,
    stream: &mut BufStream<TcpStream>,
) -> Result<(), SolutionError> {
    if let Some(msg) = solution.welcome() {
        if !msg.is_empty() {
            stream
                .write_all(msg.as_slice())
                .await
                .map_err(|_| SolutionError::Write)?;

            stream.flush().await.map_err(|_| SolutionError::Write)?;
        }
    }

    Ok(())
}

/// Custom Action type that is returned by the `Protocol`
///
/// This is used to define the action to be taken by the server
#[derive(Debug, PartialEq)]
pub enum Action {
    Broadcast(Vec<u8>),
    Disconnect(Vec<u8>),
    Reply(Vec<u8>),
    Send(SocketAddr, Vec<u8>),
}

/// Processes a connection
///
/// Returns a `Result` which contains the number of processed bytes on the
/// success path and a custom defined error `SolutionError`
async fn process(
    state: Arc<Mutex<SharedState>>,
    solution: BudgetChatSolution,
    stream: TcpStream,
) -> Result<(), SolutionError> {
    let mut solution = solution;
    let mut stream = BufStream::new(stream);
    let mut line = vec![];
    let mut should_continue = true;

    // Get address of peer
    let addr = stream
        .get_ref()
        .peer_addr()
        .map_err(|_| SolutionError::Read)?;

    // Create a channel for the peer
    let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();

    // Add entry to this peer in the shared state
    state.lock().await.peers.insert(addr, (None, tx));

    // Send welcome message to the peer
    welcome_client(&mut solution, &mut stream).await?;

    // Loop until no bytes are read
    while should_continue {
        // Wait for messages from other peers or parse the received request
        tokio::select! {
            // If there's a message from another peer
            Some(msg) = rx.recv() => {
                // Process the received peer message
                let response = match solution.process_peer_msg(&msg) {
                    Ok(Action::Broadcast(arr)) => {
                        state.lock().await.broadcast(addr, arr.as_slice()).await;
                        vec![]
                    }
                    Ok(Action::Disconnect(arr)) => {
                        should_continue = false;
                        arr
                    }
                    Ok(Action::Reply(arr)) => arr,
                    Ok(Action::Send(to, arr)) => {
                        state.lock().await.send(to, addr, arr.as_slice()).await;
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
            }
            // If there's a request incoming
            result = read_until_del(&mut stream, &mut line, solution.get_delimiter()) => {
                let read_len = result?;

                if read_len > 0 {
                    // Process the received request/line
                    let response = match solution.process_request(&line) {
                        Ok(Action::Broadcast(arr)) => {
                            state.lock().await.broadcast(addr, arr.as_slice()).await;
                            vec![]
                        }
                        Ok(Action::Disconnect(arr)) => {
                            should_continue = false;
                            arr
                        }
                        Ok(Action::Reply(arr)) => arr,
                        Ok(Action::Send(to, arr)) => {
                            state.lock().await.send(to, addr, arr.as_slice()).await;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_request() {
        let mut solution = BudgetChatSolution;
        let line = b"hello world\n";
        let res = solution.process_request(line);
        assert_eq!(res.is_ok(), true);
        assert_eq!(res.unwrap(), line.to_vec());
    }
}
