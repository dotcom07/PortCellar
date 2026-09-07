pub type Result<T> = std::result::Result<T, PortCellarError>;

#[derive(Debug)]
pub enum PortCellarError {
    Io(std::io::Error),
    Message(String),
}

impl std::fmt::Display for PortCellarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PortCellarError::Io(error) => write!(f, "{error}"),
            PortCellarError::Message(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for PortCellarError {}

impl From<std::io::Error> for PortCellarError {
    fn from(error: std::io::Error) -> Self {
        PortCellarError::Io(error)
    }
}
