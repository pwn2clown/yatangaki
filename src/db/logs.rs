use super::DbError;
use crate::proxy::{PacketId, ProxyId};
use http_body_util::Full;
use hyper::body::Bytes;
use rusqlite::Connection;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

static DB: LazyLock<Mutex<Option<LogsDb>>> = LazyLock::new(|| Mutex::new(None));

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

pub struct HttpLogRow {
    pub request_summary: PacketSummary,
    pub request_body: Vec<u8>,
    pub request_headers: HashMap<String, String>,
    pub response: Option<HttpResponseLogRow>,
}

pub struct HttpResponseLogRow {
    pub status_code: usize,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

pub struct LogsDb {
    conn: Connection,
    packet_index: usize,
}

pub fn create_project_db(name: &str) -> Result<(), Box<dyn Error>> {
    let mut full_config_dir_buf: PathBuf = [&env::var("HOME").unwrap(), super::CONFIG_DIR, name]
        .iter()
        .collect();

    let full_config_dir = full_config_dir_buf.as_path();
    if !full_config_dir.exists() {
        fs::create_dir_all(full_config_dir)?;
    }
    full_config_dir_buf.push("network_logs.db");
    let conn = Connection::open(full_config_dir_buf.as_path().to_str().unwrap())?;

    conn.execute_batch(&fs::read_to_string("./schema/network_logs.sql")?)?;

    let packet_index = {
        let mut stmt = conn.prepare("SELECT MAX(packet_id) FROM requests")?;
        let mut rows = stmt.query([])?;
        let p = match rows.next().unwrap() {
            Some(row) => {
                let max_packet_id: usize = row
                    .get::<usize, usize>(0)
                    .map(|max| max + 1)
                    .unwrap_or_default();
                max_packet_id
            }
            None => 0,
        };
        p
    };

    let _ = DB.lock().unwrap().insert(LogsDb { conn, packet_index });
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
    response: &hyper::Response<Full<Bytes>>,
    body: Bytes,
    packet_id: PacketId,
) -> Result<(), DbError> {
    let Some(ref mut db) = *DB.lock().unwrap() else {
        return Err(DbError::NoDatabaseSelected);
    };

    db.conn
        .execute(
            "INSERT INTO responses (packet_id, status, body) VALUES (?1, ?2, ?3)",
            (packet_id, response.status().as_u16(), body.to_vec()),
        )
        .map_err(DbError::SqliteError)?;

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

pub fn get_full_row(packet_id: PacketId) -> Result<Option<HttpLogRow>, DbError> {
    let Some(ref mut db) = *DB.lock().unwrap() else {
        return Err(DbError::NoDatabaseSelected);
    };

    let mut stmt = db
        .conn
        .prepare("SELECT key, value FROM request_headers WHERE packet_id = ?1;")
        .map_err(DbError::SqliteError)?;

    let mut rows = stmt.query([packet_id]).map_err(DbError::SqliteError)?;

    let mut headers = HashMap::default();
    while let Some(row) = rows.next().map_err(DbError::SqliteError)? {
        headers.insert(row.get_unwrap(0), row.get_unwrap(1));
    }

    let mut stmt = db
        .conn
        .prepare("SELECT packet_id, proxy_id, method, authority, path, query, body FROM requests WHERE packet_id = ?1;")
        .map_err(DbError::SqliteError)?;
    let mut rows = stmt.query([packet_id]).map_err(DbError::SqliteError)?;

    Ok(match rows.next().map_err(DbError::SqliteError)? {
        Some(row) => {
            let http_log = HttpLogRow {
                request_summary: PacketSummary {
                    packet_id: row.get(0).unwrap(),
                    proxy_id: row.get(1).unwrap(),
                    method: row.get(2).unwrap(),
                    authority: row.get(3).unwrap(),
                    path: row.get(4).unwrap(),
                    query: row.get(5).unwrap_or_default(),
                },
                request_body: row.get_unwrap(6),
                request_headers: headers,
                response: None,
            };

            Some(http_log)
        }
        None => None,
    })
}
