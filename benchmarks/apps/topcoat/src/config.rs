use std::{env, path::PathBuf};

/// Server settings read from the benchmark environment variables.
///
/// `PORT` is read by [`topcoat::start`] itself.
#[derive(Debug)]
pub struct Config {
    pub data_dir: PathBuf,
    /// Number of worker threads, or `None` for the Tokio default of one per core.
    pub threads: Option<usize>,
    pub compression: bool,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let data_dir = match env::var_os("BENCH_DATA_DIR") {
            Some(value) => PathBuf::from(value),
            None => PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data")),
        };
        let threads = match env::var("BENCH_THREADS") {
            Ok(value) => match value.parse() {
                Ok(0) => None,
                Ok(threads) => Some(threads),
                Err(_) => return Err(format!("invalid BENCH_THREADS {value:?}")),
            },
            Err(_) => None,
        };
        let compression = match env::var("BENCH_COMPRESSION").as_deref() {
            Ok("on") => true,
            Ok("off") | Err(_) => false,
            Ok(value) => return Err(format!("invalid BENCH_COMPRESSION {value:?}")),
        };
        Ok(Self {
            data_dir,
            threads,
            compression,
        })
    }
}
