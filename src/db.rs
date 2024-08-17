use crate::proxy::{PacketId, ProxyId};
use http_body_util::{BodyExt, Collected};
use hyper::body::Bytes;
use rusqlite::Connection;
use std::sync::{LazyLock, Mutex};

static DB: LazyLock<Mutex<Option<Db>>> = LazyLock::new(|| Mutex::new(None));

pub struct PacketSummary {
    pub packet_id: PacketId,
    pub proxy_id: usize,
    pub method: String,
    pub authority: String,
    pub path: String,
    pub query: String,
    //  TODO: add optionnal response elements
    //  status_code: Option<usize>,
}

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
            "CREATE TABLE IF NOT EXISTS requests (
                packet_id INTEGER UNIQUE NOT NULL,
                proxy_id INTEGER NOT NULL,
                method TEXT NOT NULL,
                authority TEXT NOT NULL,
                path TEXT NOT NULL,
                query TEXT,
                body BLOB
            )",
            (),
        )
        .map_err(DbError::SqliteError)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS responses (
                packet_id INTEGER UNIQUE NOT NULL,
                status INTEGER NOT NULL,
                body BLOB
            )",
            (),
        )
        .map_err(DbError::SqliteError)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS request_headers (
                packet_id INTEGER NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL
            )",
            (),
        )
        .map_err(DbError::SqliteError)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS response_headers (
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
    ) -> Result<PacketId, DbError> {
        let Some(ref mut db) = *DB.lock().unwrap() else {
            return Err(DbError::NoDatabaseSelected);
        };

        let uri = request.uri();
        db.conn
            .execute(
                "INSERT INTO requests (packet_id, proxy_id, method, authority, path, query, body) VALUES(
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7
                )",
                (
                    db.packet_index,
                    proxy_id,
                    request.method().as_str(),
                    uri.authority().unwrap().to_string(),
                    uri.path(),
                    uri.query(),
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
        Ok(db.packet_index)
    }

    pub fn insert_response(
        response: &hyper::Response<Collected<Bytes>>,
        packet_id: PacketId,
    ) -> Result<(), DbError> {
        let Some(ref mut db) = *DB.lock().unwrap() else {
            return Err(DbError::NoDatabaseSelected);
        };

        /*
        db.conn
            .execute(
                "INSERT INTO responses (packet_id, status, body) VALUES (?1, ?2, ?3)",
                (
                    packet_id,
                    response.status().as_u16(),
                    response.body().collect().,
                ),
            )
            .map_err(DbError::SqliteError)?;
        */

        Ok(())
    }

    pub fn get_packets_summary() -> Result<Vec<PacketSummary>, DbError> {
        let Some(ref mut db) = *DB.lock().unwrap() else {
            return Err(DbError::NoDatabaseSelected);
        };

        let mut packet_summaries = vec![];
        let mut stmt = db
            .conn
            .prepare("SELECT packet_id, proxy_id, method, authority, path, query FROM requests;")
            .map_err(DbError::SqliteError)?;

        let mut rows = stmt.query([]).map_err(DbError::SqliteError)?;

        while let Some(row) = rows.next().map_err(DbError::SqliteError)? {
            packet_summaries.push(PacketSummary {
                packet_id: row.get(0).unwrap(),
                proxy_id: row.get(1).unwrap(),
                method: row.get(2).unwrap(),
                authority: row.get(3).unwrap(),
                path: row.get(4).unwrap(),
                query: row.get(5).unwrap_or_default(),
            })
        }

        Ok(packet_summaries)
    }
}
