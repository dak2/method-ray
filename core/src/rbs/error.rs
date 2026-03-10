use std::fmt;

#[derive(Debug)]
pub enum RbsError {
    RbsNotInstalled,
    LoadError(String),
    ParseError(String),
    MagnusError(magnus::Error),
}

impl fmt::Display for RbsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RbsError::RbsNotInstalled => {
                write!(f, "RBS gem not found. Please install: gem install rbs")
            }
            RbsError::LoadError(msg) => write!(f, "Failed to load RBS environment: {}", msg),
            RbsError::ParseError(msg) => write!(f, "Failed to parse RBS type: {}", msg),
            RbsError::MagnusError(e) => write!(f, "Magnus error: {}", e),
        }
    }
}

impl std::error::Error for RbsError {}

impl From<magnus::Error> for RbsError {
    fn from(err: magnus::Error) -> Self {
        RbsError::MagnusError(err)
    }
}

impl From<RbsError> for magnus::Error {
    fn from(err: RbsError) -> Self {
        let ruby = unsafe { magnus::Ruby::get_unchecked() };
        magnus::Error::new(ruby.exception_runtime_error(), err.to_string())
    }
}
