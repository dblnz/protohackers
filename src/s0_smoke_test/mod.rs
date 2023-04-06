use crate::solution::{ProtoHSolution, SolutionError};

#[derive(Debug)]
pub struct SmokeTestSolution;

impl ProtoHSolution for SmokeTestSolution {
    fn process_request(&mut self, line: &[u8]) -> Result<Vec<u8>, SolutionError> {
        Ok(line.to_vec())
    }
}
