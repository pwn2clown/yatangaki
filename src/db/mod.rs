pub mod config;
pub mod logs;

const CONFIG_DIR: &str = ".yatangaki";

#[derive(Debug)]
pub enum DbError {
    NoDatabaseSelected,
    SqliteError(rusqlite::Error),
}
