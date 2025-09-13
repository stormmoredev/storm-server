#[cfg(test)]
mod tests  {
    use crate::conf::args::args_parser::{ArgKind, ArgsParser};

    #[test]
    fn parse_args_should_return_filename() {
        let args = ["", "-f", "lb.conf"]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();

        let mut parser = ArgsParser::new();
        parser.add(ArgKind::Value(String::from("-f")));

        let result = parser.parse(&args);

        assert!(result.is_ok());
    }

    #[test]
    fn parse_args_should_return_error() {
        let args = ["", "-d", "lb.conf"]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();

        let mut parser = ArgsParser::new();
        parser.add(ArgKind::Value(String::from("-f")));

        let result = parser.parse(&args);

        assert!(result.is_err());
    }
}