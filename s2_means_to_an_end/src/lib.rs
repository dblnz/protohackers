use traits::{Protocol, RequestDelimiter, SolutionError};
use std::collections::HashMap;

/// Means To An End
///
/// Clients will connect to your server using TCP.
/// Each client tracks the price of a different asset.
///  Clients send messages to the server that either insert or query the prices.
///
/// Each connection from a client is a separate session.
/// Each session's data represents a different asset, so each session can only query the data supplied by itself.
///
/// MESSAGE FORMAT
///
/// Each message from a client is 9 bytes long. Clients can send multiple messages per connection.
/// Messages are not delimited by newlines or any other character:
/// you'll know where one message ends and the next starts because they are always 9 bytes.
///
/// Byte:  |  0  |  1     2     3     4  |  5     6     7     8  |
/// Type:  |char |         int32         |         int32         |
///
/// The first byte of a message is a character indicating its type. This will be an ASCII uppercase 'I' or 'Q' character,
///  indicating whether the message inserts or queries prices, respectively.
///
/// The next 8 bytes are two signed two's complement 32-bit integers in network byte order (big endian),
///  whose meaning depends on the message type. We'll refer to these numbers as int32, but note this may differ
///  from your system's native int32 type (if any), particularly with regard to byte order.
///
/// INSERT
///
/// An insert message lets the client insert a timestamped price.
///
/// Byte:  |  0  |  1     2     3     4  |  5     6     7     8  |
/// Type:  |char |         int32         |         int32         |
/// Value: | 'I' |       timestamp       |         price         |
///
/// The first int32 is the timestamp, in seconds since 00:00, 1st Jan 1970.
/// The second int32 is the price, in pennies, of this client's asset, at the given timestamp.
/// Note that:
///     Insertions may occur out-of-order.
///     While rare, prices can go negative.
///     Behaviour is undefined if there are multiple prices with the same timestamp from the same client.
///
/// For example, to insert a price of 101 pence at timestamp 12345, a client would send:
/// Hexadecimal: 49    00 00 30 39    00 00 00 65
/// Decoded:      I          12345            101
///
/// (Remember that you'll receive 9 raw bytes, rather than ASCII text representing hex-encoded data).
/// Behaviour is undefined if the type specifier is not either 'I' or 'Q'.
///
/// QUERY
///
/// A query message lets the client query the average price over a given time period.
///
/// The message format is:
/// Byte:  |  0  |  1     2     3     4  |  5     6     7     8  |
/// Type:  |char |         int32         |         int32         |
/// Value: | 'Q' |        mintime        |        maxtime        |
///
/// The first int32 is mintime, the earliest timestamp of the period.
/// The second int32 is maxtime, the latest timestamp of the period.
///
/// The server must compute the mean of the inserted prices with timestamps T, mintime <= T <= maxtime
/// (i.e. timestamps in the closed interval [mintime, maxtime]).
///  If the mean is not an integer, it is acceptable to round either up or down, at the server's discretion.
///
/// The server must then send the mean to the client as a single int32.
///
/// If there are no samples within the requested period, or if mintime comes after maxtime, the value returned must be 0.
///
/// For example, to query the mean price between T=1000 and T=100000, a client would send:
/// Hexadecimal: 51    00 00 03 e8    00 01 86 a0
/// Decoded:      Q           1000         100000
///
/// And if the mean price during this time period were 5107 pence, the server would respond:
///
/// Hexadecimal: 00 00 13 f3
/// Decoded:            5107
///
/// (Remember that you'll receive 9 raw bytes, and send 4 raw bytes, rather than ASCII text representing hex-encoded data).
///
/// EXAMPLE SESSION
///
/// In this example, "-->" denotes messages from the server to the client, and "<--" denotes messages from the client to the server.
///
///     Hexadecimal:                 Decoded:
/// <-- 49 00 00 30 39 00 00 00 65   I 12345 101
/// <-- 49 00 00 30 3a 00 00 00 66   I 12346 102
/// <-- 49 00 00 30 3b 00 00 00 64   I 12347 100
/// <-- 49 00 00 a0 00 00 00 00 05   I 40960 5
/// <-- 51 00 00 30 00 00 00 40 00   Q 12288 16384
/// --> 00 00 00 65                  101
///
/// The client inserts (timestamp,price) values: (12345,101), (12346,102), (12347,100), and (40960,5).
/// The client then queries between T=12288 and T=16384. The server computes the mean price during this period,
/// which is 101, and sends back 101.
///
/// OTHER REQUIREMENTS
///
/// Make sure you can handle at least 5 simultaneous clients.
///
/// Where a client triggers undefined behaviour, the server can do anything it likes for that client,
/// but must not adversely affect other clients that did not trigger undefined behaviour.
#[derive(Debug)]
pub struct MeansToAnEndSolution {
    db: HashMap<i32, i32>,
}

#[derive(Debug, PartialEq)]
enum MessageType {
    Insert(i32, i32),
    Query(i32, i32),
}

impl MessageType {
    fn from_bytes(line: &[u8]) -> Option<MessageType> {
        if 9 == line.len() {
            let w1 = ((line[1] as i32) << 24)
                + ((line[2] as i32) << 16)
                + ((line[3] as i32) << 8)
                + line[4] as i32;
            let w2 = ((line[5] as i32) << 24)
                + ((line[6] as i32) << 16)
                + ((line[7] as i32) << 8)
                + line[8] as i32;

            if line[0] == b'I' {
                Some(MessageType::Insert(w1, w2))
            } else if line[0] == b'Q' {
                Some(MessageType::Query(w1, w2))
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl MeansToAnEndSolution {
    pub fn new() -> Self {
        Self { db: HashMap::new() }
    }

    fn insert(&mut self, ts: i32, price: i32) {
        self.db.insert(ts, price);
    }

    fn query(&self, min: i32, max: i32) -> Vec<u8> {
        let mut res = vec![0; 4];

        let mut sum = 0i64;
        let mut n = 0i64;

        let mut sorted = self.db.iter().collect::<Vec<(&i32, &i32)>>();

        sorted.sort_by(|a, b| b.0.cmp(a.0));

        (sum, n) = sorted.iter().fold((sum, n), |mut acc, item| {
            if *item.0 >= min && *item.0 <= max {
                acc.0 += *item.1 as i64;
                acc.1 += 1;
            }
            acc
        });

        let mean = if n > 0 { (sum / n) as i32 } else { 0 };

        // Write into vec in LE order
        res[3] = mean as u8;
        res[2] = (mean >> 8) as u8;
        res[1] = (mean >> 16) as u8;
        res[0] = (mean >> 24) as u8;

        res
    }
}

impl Protocol for MeansToAnEndSolution {
    fn get_delimiter() -> RequestDelimiter {
        RequestDelimiter::NoOfBytes(9)
    }

    fn process_request(&mut self, line: &[u8]) -> Result<Vec<u8>, SolutionError> {
        match MessageType::from_bytes(line) {
            Some(MessageType::Insert(ts, price)) => {
                self.insert(ts, price);
                Ok(vec![])
            }
            Some(MessageType::Query(min, max)) => Ok(self.query(min, max)),
            None => Err(SolutionError::Request(b"malformed".to_vec()))?,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_messagetype_deserialize_invalid_bytes_length_error() {
        let line = vec![0; 10];

        assert_eq!(MessageType::from_bytes(&line), None);
    }

    #[test]
    fn test_message_deserialize_success() {
        let line = vec!['I' as u8, 0x00, 0x00, 0x00, 0x0A, 0x0A, 0x00, 0x00, 0x00];
        let mt = MessageType::from_bytes(&line);

        assert_eq!(mt.is_some(), true);
        let mt = mt.unwrap();
        assert_eq!(mt, MessageType::Insert(0x0A, 0x0A000000));
    }

    #[test]
    fn test_complete_session_success() {
        let mut sol = MeansToAnEndSolution::new();
        let line_0 = vec![0x49, 0x00, 0x00, 0x30, 0x39, 0x00, 0x00, 0x00, 0x65];
        let line_1 = vec![0x49, 0x00, 0x00, 0x30, 0x3A, 0x00, 0x00, 0x00, 0x66];
        let line_2 = vec![0x49, 0x00, 0x00, 0x30, 0x3B, 0x00, 0x00, 0x00, 0x64];
        let line_3 = vec![0x49, 0x00, 0x00, 0xA0, 0x00, 0x00, 0x00, 0x00, 0x05];
        let line_4 = vec![0x51, 0x00, 0x00, 0x30, 0x00, 0x00, 0x00, 0x40, 0x00];

        assert_eq!(sol.process_request(&line_0), Ok(vec![]));
        assert_eq!(sol.process_request(&line_1), Ok(vec![]));
        assert_eq!(sol.process_request(&line_2), Ok(vec![]));
        assert_eq!(sol.process_request(&line_3), Ok(vec![]));
        assert_eq!(
            sol.process_request(&line_4),
            Ok(vec![0x00, 0x00, 0x00, 0x65])
        );
    }

    #[test]
    fn test_check_repr_success() {
        let sum = 102452436047i64;
        let n = 1168;

        let mean = if n > 0 { (sum / n) as i32 } else { 0 };

        // Write into vec in LE order
        assert_eq!(87716126, mean);
    }
}
