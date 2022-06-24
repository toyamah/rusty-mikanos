use alloc::collections::VecDeque;
use alloc::string::String;

pub(crate) enum Direction {
    Up,
    Down,
}

#[derive(Debug)]
pub(crate) struct CommandHistory {
    //       New      Old
    // index   0 1 2 3
    history: VecDeque<String>,
    pointing_index: Option<usize>,
}

impl CommandHistory {
    const MAX: usize = 8;

    pub(crate) fn new() -> CommandHistory {
        Self {
            history: VecDeque::with_capacity(Self::MAX),
            pointing_index: None,
        }
    }

    pub(crate) fn up(&mut self) -> &str {
        if self.history.is_empty() {
            self.pointing_index = None;
            return "";
        }

        self.pointing_index = match self.pointing_index {
            None => Some(0), // return the newest
            Some(i) if i + 1 < self.history.len() => Some(i + 1),
            Some(_) => Some(self.history.len() - 1), // return the oldest
        };
        self.pointing_index
            .map(|i| self.history.get(i).unwrap())
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    pub(crate) fn down(&mut self) -> &str {
        self.pointing_index = match self.pointing_index {
            Some(i) if i > 0 => Some(i - 1),
            Some(_) => None, // return None because of no more new command
            None => None,
        };

        self.pointing_index
            .map(|i| self.history.get(i).unwrap())
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    pub(crate) fn push(&mut self, command: String) {
        self.pointing_index = None;

        if command.is_empty() {
            return;
        }

        if self.history.len() == Self::MAX {
            self.history.pop_back().unwrap();
        }
        self.history.push_front(command);
    }
}

#[cfg(test)]
mod command_history_tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn up_should_return_empty_if_it_has_no_history() {
        let mut history = CommandHistory::new();
        assert_eq!(history.up(), "");
    }

    #[test]
    fn up_should_return_next_old_comand_if_it_has_history() {
        let mut history = CommandHistory::new();
        history.push("a".to_string());
        history.push("b".to_string());
        history.push("c".to_string());

        assert_eq!(history.up(), "c");
        assert_eq!(history.up(), "b");
        assert_eq!(history.up(), "a");
        assert_eq!(history.up(), "a");
        assert_eq!(history.up(), "a");
    }

    #[test]
    fn down_should_return_empty_if_it_has_no_history() {
        let mut history = CommandHistory::new();
        assert_eq!(history.down(), "");
    }

    #[test]
    fn down_should_return_next_new_command_if_it_has_history() {
        let mut history = CommandHistory::new();
        history.push("a".to_string());
        history.push("b".to_string());
        history.push("c".to_string());

        history.up(); // c
        history.up(); // b
        history.up(); // a
        history.up(); // a and pointing index should not be changed.

        assert_eq!(history.down(), "b");
        assert_eq!(history.down(), "c");
        assert_eq!(history.down(), "");
        assert_eq!(history.down(), "");
        assert_eq!(history.down(), "");
    }

    #[test]
    fn push_should_reset_index() {
        let mut history = CommandHistory::new();
        history.push("a".to_string());
        history.push("b".to_string());
        history.push("c".to_string());

        history.up(); // c
        history.up(); // b
        history.up(); // a

        history.push("d".to_string());

        // up should return the newest command because of resetting the index.
        assert_eq!(history.up(), "d")
    }

    #[test]
    fn push_should_remove_oldest_if_history_is_full() {
        let mut history = CommandHistory::new();
        for i in 0..CommandHistory::MAX {
            history.push(i.to_string());
        }

        history.push(CommandHistory::MAX.to_string());

        assert_eq!(
            history.history.front().unwrap(),
            &CommandHistory::MAX.to_string()
        );
        assert_eq!(history.history.back().unwrap(), &"1".to_string()); // not "0"
    }
}
