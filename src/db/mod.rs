pub mod config;
pub mod logs;

#[derive(Debug)]
pub enum DbError {
    NoDatabaseSelected,
    SqliteError(rusqlite::Error),
}
