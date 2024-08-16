use hyper::body::Bytes;
use rusqlite::Connection;
use std::sync::{LazyLock, Mutex};

use crate::proxy::ProxyId;

static DB: LazyLock<Mutex<Option<Db>>> = LazyLock::new(|| Mutex::new(None));

pub struct Db {
    conn: Connection,
    packet_index: usize,
}

#[derive(Debug)]
pub enum DbError {
    NoDatabaseSelected,
    SqliteError(rusqlite::Error),
}

impl Db {
    pub fn create_project_db(name: &str) -> Result<(), DbError> {
        let conn = Connection::open(format!("./{name}.db")).map_err(DbError::SqliteError)?;
        conn.execute(
            "CREATE TABLE request (
                packet_id INTEGER UNIQUE NOT NULL,
                proxy_id INTEGER NOT NULL,
                method TEXT NOT NULL,
                url TEXT NOT NULL,
                body BLOB
            )",
            (),
        )
        .map_err(DbError::SqliteError)?;

        conn.execute(
            "CREATE TABLE request_headers (
                packet_id INTEGER NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL
            )",
            (),
        )
        .map_err(DbError::SqliteError)?;

        let _ = DB.lock().unwrap().insert(Db {
            conn,
            packet_index: 0,
        });

        Ok(())
    }

    pub fn insert_request(
        request: &hyper::Request<Bytes>,
        proxy_id: ProxyId,
    ) -> Result<(), DbError> {
        let Some(ref mut db) = *DB.lock().unwrap() else {
            return Err(DbError::NoDatabaseSelected);
        };

        db.conn
            .execute(
                "INSERT INTO request (packet_id, proxy_id, method, url, body) VALUES(
                    ?1, ?2, ?3, ?4, ?5
                )",
                (
                    db.packet_index,
                    proxy_id,
                    request.method().as_str(),
                    request.uri().to_string(),
                    request.body().to_vec(),
                ),
            )
            .map_err(DbError::SqliteError)?;

        let mut pstmt = db
            .conn
            .prepare("INSERT INTO request_headers (packet_id, key, value) VALUES (?1, ?2, ?3)")
            .map_err(DbError::SqliteError)?;

        for (key, value) in request.headers() {
            pstmt
                .execute((db.packet_index, key.as_str(), value.to_str().unwrap()))
                .map_err(DbError::SqliteError)?;
        }

        db.packet_index += 1;
        Ok(())
    }
}
