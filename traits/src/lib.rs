use std::marker::Sync;

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
    fn process_request(&mut self, line: &[u8]) -> Result<Vec<u8>, SolutionError>;
}
