use server::{Action, Protocol, RequestDelimiter, SolutionError};

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
        Self { name: None, members: vec![], state: ChatState::Joined }
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
}

impl Protocol for BudgetChatSolution {
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
                Ok(Action::Reply(b"Welcome to budgetchat! What shall I call you?\n".to_vec()))
            }
            ChatState::Sync => {
                // Advance state
                self.next_state();
                Ok(Action::Reply(b"* The room contains: bob, charlie, dave\n".to_vec()))
            }
            ChatState::Welcome => {
                // Advance state
                self.next_state();
                // Set name
                let name = std::str::from_utf8(line).unwrap().trim();
                self.set_name(name.clone());
                // Add member
                self.add_member(name);
                Ok(Action::Reply(b"* The room contains: bob, charlie, dave\n".to_vec()))
            }
            ChatState::BroadcastName => {
                // Advance state
                self.next_state();
                // Broadcast name
                Ok(Action::Broadcast(format!("* {} has entered the room\n", self.name.as_ref().unwrap()).into_bytes()))
            }
            ChatState::ShowMembers => {
                // Advance state
                self.next_state();
                // Show members
                Ok(Action::Reply(format!("* The room contains: {}\n", self.members.join(", ")).into_bytes()))
            }
            ChatState::Chat => {
                // Advance state
                self.next_state();
                // Broadcast message
                Ok(Action::Broadcast(format!("[{}] {}\n", self.name.as_ref().unwrap(), std::str::from_utf8(line).unwrap().trim()).into_bytes()))
            }
        }
    }

    /// method to process each received peer message
    fn process_peer_msg(&mut self, line: &[u8]) -> Result<Action, SolutionError> {
        unimplemented!()
    }
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
