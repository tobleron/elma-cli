use crate::*;

#[derive(Debug, Clone)]
pub(crate) enum InputMode {
    Chat(String),
    Command(String),
    Shell(String),
    File(String),
    Search(String),
}

pub(crate) fn parse_input(input: &str) -> InputMode {
    let input = input.trim();
    if input.is_empty() {
        return InputMode::Chat(String::new());
    }
    if input.starts_with('/') {
        InputMode::Command(input[1..].to_string())
    } else if input.starts_with('!') {
        InputMode::Shell(input[1..].to_string())
    } else if input.starts_with('@') {
        InputMode::File(input[1..].to_string())
    } else if input.starts_with('?') {
        InputMode::Search(input[1..].to_string())
    } else {
        InputMode::Chat(input.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_chat() {
        assert!(matches!(parse_input("hello"), InputMode::Chat(s) if s == "hello"));
    }

    #[test]
    fn test_parse_command() {
        assert!(matches!(parse_input("/help"), InputMode::Command(s) if s == "help"));
    }

    #[test]
    fn test_parse_shell() {
        assert!(matches!(parse_input("!ls -la"), InputMode::Shell(s) if s == "ls -la"));
    }

    #[test]
    fn test_parse_file() {
        assert!(matches!(parse_input("@Cargo.toml"), InputMode::File(s) if s == "Cargo.toml"));
    }

    #[test]
    fn test_parse_search() {
        assert!(matches!(parse_input("?rust async"), InputMode::Search(s) if s == "rust async"));
    }

    #[test]
    fn test_parse_empty() {
        assert!(matches!(parse_input(""), InputMode::Chat(s) if s.is_empty()));
    }
}
