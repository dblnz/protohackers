use server::{Action, Protocol, RequestDelimiter, SolutionError};

#[derive(Debug, Default)]
pub struct BudgetChatSolution;

impl Protocol for BudgetChatSolution {
    /// method to get the delimiter between two requests
    fn get_delimiter(&self) -> RequestDelimiter {
        RequestDelimiter::UntilChar(b'\n')
    }

    /// method to process each received request/line
    fn process_request(&mut self, line: &[u8]) -> Result<Action, SolutionError> {
        Ok(Action::Broadcast(line.to_vec()))
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
