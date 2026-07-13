use topcoat_core_grammar::pretty::FormatError;

#[derive(Debug)]
pub enum Error {
    Glob(glob::GlobError),
    Pattern(glob::PatternError),
    Io(std::io::Error),
    Syntax { errors: Vec<FormatError> },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Glob(inner) => write!(f, "{inner}")?,
            Self::Pattern(inner) => write!(f, "{inner}")?,
            Self::Io(inner) => write!(f, "{inner}")?,
            Self::Syntax { errors } => {
                write!(f, "syntax errors while formatting view macro:")?;
                for error in errors {
                    let start = error.start();
                    let line = start.line;
                    let column = start.column;
                    write!(f, "\n  at line {line} column {column}: {error}")?;
                }
            }
        }
        Ok(())
    }
}

impl std::error::Error for Error {}

impl From<glob::GlobError> for Error {
    fn from(value: glob::GlobError) -> Self {
        Self::Glob(value)
    }
}

impl From<glob::PatternError> for Error {
    fn from(value: glob::PatternError) -> Self {
        Self::Pattern(value)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<FormatError> for Error {
    fn from(value: FormatError) -> Self {
        Self::Syntax {
            errors: vec![value],
        }
    }
}

impl From<Vec<FormatError>> for Error {
    fn from(value: Vec<FormatError>) -> Self {
        Self::Syntax { errors: value }
    }
}
