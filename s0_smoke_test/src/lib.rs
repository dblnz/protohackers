use server::{Protocol, SolutionError};

/// Smoke Test - TCP Echo Service
///
/// This Protocol implies replying to the requests with
/// the same data received
#[derive(Debug, Default)]
pub struct SmokeTestSolution;

impl SmokeTestSolution {
    /// Constructor for the SmokeTestSolution
    pub fn new() -> Self {
        Self::default()
    }
}

/// Implementation of the Protocol trait for SmokeTestSolution
/// This is where the custom logic for the solution is implemented
impl Protocol for SmokeTestSolution {
    /// Custom method to process each received request/line
    fn process_request(&mut self, line: &[u8]) -> Result<Vec<u8>, SolutionError> {
        Ok(line.to_vec())
    }
}

