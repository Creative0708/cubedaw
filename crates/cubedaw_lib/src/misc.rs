use std::io;

pub fn stringify_ciborium_error(err: ciborium::de::Error<io::Error>) -> String {
    use ciborium::de::Error;
    match err {
        Error::Io(err) => format!("Io error: {}", err.to_string()),
        Error::Syntax(pos) => format!("Syntax error at position {pos}"),
        Error::Semantic(None, msg) => format!("Syntax error: {msg}"),
        Error::Semantic(Some(pos), msg) => format!("Syntax error at position {pos}: {msg}"),
        Error::RecursionLimitExceeded => format!("Recursion limit exceeded"),
    }
}
