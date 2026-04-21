#[derive(Debug)]
pub enum Error {
    Glob(glob::GlobError),
    Pattern(glob::PatternError),
    Io(std::io::Error),
    Syntax { errors: Vec<syn::Error> },
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
                    let span = error.span();
                    let start = span.start();
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

impl From<syn::Error> for Error {
    fn from(value: syn::Error) -> Self {
        Self::Syntax {
            errors: vec![value],
        }
    }
}

impl From<Vec<syn::Error>> for Error {
    fn from(value: Vec<syn::Error>) -> Self {
        Self::Syntax { errors: value }
    }
}
