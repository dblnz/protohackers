use traits::{Protocol, SolutionError};

/// Smoke Test - TCP Echo Service
///
/// This Protocol implies replying to the requests with
/// the same data received
#[derive(Debug, Default)]
pub struct SmokeTestSolution;

impl SmokeTestSolution {
    pub fn new() -> Self {
        Self
    }
}

impl Protocol for SmokeTestSolution {
    fn process_request(&mut self, line: &[u8]) -> Result<Vec<u8>, SolutionError> {
        Ok(line.to_vec())
    }
}

