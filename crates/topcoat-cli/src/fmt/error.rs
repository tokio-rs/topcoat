#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Syntax { errors: Vec<syn::Error> },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(inner) => write!(f, "{inner}")?,
            Self::Syntax { errors } => {
                write!(f, "syntax errors while formatting view macro:")?;
                for error in errors {
                    write!(f, "\n{error}")?;
                }
            }
        }
        Ok(())
    }
}

impl std::error::Error for Error {}

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
