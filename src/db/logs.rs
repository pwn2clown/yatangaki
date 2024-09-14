use crate::proxy::types::{PacketId, ProxyId};
use http_body_util::Full;
use hyper::body::Bytes;
use rusqlite::Connection;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

static PROJECT_DB: LazyLock<Mutex<Connection>> =
    LazyLock::new(|| Mutex::new(Connection::open_in_memory().unwrap()));

pub struct HttpLogRowMetadata {
    pub packet_id: PacketId,
    pub proxy_id: usize,
    pub method: String,
    pub authority: String,
    pub path: String,
    pub query: String,
    pub status: Option<usize>,
}

pub struct HttpPacketContent {
    pub body: Vec<u8>,
    pub headers: HashMap<String, String>,
}

pub struct HttpLogRow {
    pub metadata: HttpLogRowMetadata,
    pub request: HttpPacketContent,
    pub response: Option<HttpPacketContent>,
}

impl HttpLogRow {
    pub fn request_as_str(&self) -> String {
        //  TODO: add query after path if any
        let mut raw_request = format!("{} {} HTTP/1.1\n", self.metadata.method, self.metadata.path);
        for (key, value) in &self.request.headers {
            raw_request.push_str(&format!("{key}: {value}\n"));
        }
        raw_request.push('\n');
        raw_request.push_str(
            &String::from_utf8_lossy(&self.request.body).replace(|c: char| !c.is_ascii(), "."),
        );
        raw_request
    }

    pub fn response_as_str(&self) -> Option<String> {
        let mut raw_response = format!("HTTP/1.1 {}\n", self.metadata.status?);

        for (key, value) in &self.response.as_ref()?.headers {
            raw_response.push_str(&format!("{key}: {value}\n"));
        }

        raw_response.push('\n');
        raw_response.push_str(
            &String::from_utf8_lossy(&self.response.as_ref()?.body)
                .replace(|c: char| !c.is_ascii(), "."),
        );

        Some(raw_response)
    }
}

pub fn select_project_db(name: &str) -> Result<(), Box<dyn Error>> {
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
    *PROJECT_DB.lock().unwrap() = conn;
    Ok(())
}

pub fn insert_http_row(
    proxy_id: ProxyId,
    request_body: Bytes,
    request: http::request::Parts,
    response: Option<(hyper::Response<Full<Bytes>>, Bytes)>,
) -> Result<(), Box<dyn Error>> {
    let mut conn = PROJECT_DB.lock().unwrap();
    let packet_id: usize = {
        let mut stmt = conn.prepare_cached("SELECT MAX(packet_id) + 1 FROM requests")?;

        let packet_id = match stmt.query(())?.next()? {
            Some(row) => row.get(0).unwrap_or(0),
            None => 0,
        };
        packet_id
    };

    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO requests (packet_id, proxy_id, method, authority, path, query, body) VALUES(
            ?1, ?2, ?3, ?4, ?5, ?6, ?7
        )",
        (
            packet_id,
            proxy_id,
            request.method.as_str(),
            request.uri.authority().unwrap().to_string(),
            request.uri.path(),
            request.uri.query(),
            request_body.to_vec(),
        ),
    )?;

    {
        let mut pstmt = tx.prepare_cached(
            "INSERT INTO request_headers (packet_id, key, value) VALUES (?1, ?2, ?3)",
        )?;

        for (key, value) in request.headers.iter() {
            pstmt.execute((packet_id, key.as_str(), value.to_str().unwrap()))?;
        }
    }

    if let Some((parts, body)) = response {
        tx.execute(
            "INSERT INTO responses (packet_id, status, body) VALUES (?1, ?2, ?3)",
            (packet_id, parts.status().as_u16(), body.to_vec()),
        )?;

        let mut pstmt = tx.prepare_cached(
            "INSERT INTO response_headers (packet_id, key, value) VALUES (?1, ?2, ?3)",
        )?;

        for (key, value) in parts.headers().iter() {
            pstmt.execute((packet_id, key.as_str(), value.to_str().unwrap()))?;
        }
    }

    tx.commit()?;
    Ok(())
}

pub fn get_row_metadata() -> Result<Vec<HttpLogRowMetadata>, Box<dyn Error>> {
    let mut packet_summaries = vec![];
    let conn = PROJECT_DB.lock().unwrap();
    let mut stmt = conn.prepare_cached(
        "SELECT packet_id, proxy_id, method, authority, path, query FROM requests;",
    )?;
    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        packet_summaries.push(HttpLogRowMetadata {
            packet_id: row.get_unwrap(0),
            proxy_id: row.get_unwrap(1),
            method: row.get_unwrap(2),
            authority: row.get_unwrap(3),
            path: row.get_unwrap(4),
            query: row.get(5).unwrap_or_default(),
            status: None,
        })
    }

    Ok(packet_summaries)
}

pub fn get_full_row(packet_id: PacketId) -> Result<Option<HttpLogRow>, Box<dyn Error>> {
    let mut binding = PROJECT_DB.lock();
    let conn = binding.as_mut().unwrap();
    let tx = conn.transaction()?;

    let mut stmt =
        tx.prepare_cached("SELECT key, value FROM request_headers WHERE packet_id = ?1;")?;
    let mut rows = stmt.query([packet_id])?;

    let mut request_headers: HashMap<String, String> = HashMap::default();
    while let Some(row) = rows.next()? {
        request_headers.insert(row.get_unwrap(0), row.get_unwrap(1));
    }

    let mut stmt = tx
        .prepare_cached("SELECT packet_id, proxy_id, method, authority, path, query, body FROM requests WHERE packet_id = ?1;")?;
    let mut rows = stmt.query([packet_id])?;

    let (metadata, request_body) = match rows.next()? {
        Some(row) => (
            HttpLogRowMetadata {
                packet_id: row.get_unwrap(0),
                proxy_id: row.get_unwrap(1),
                method: row.get_unwrap(2),
                authority: row.get_unwrap(3),
                path: row.get_unwrap(4),
                query: row.get(5).unwrap_or_default(),
                status: None,
            },
            row.get(6).unwrap_or_default(),
        ),
        None => return Ok(None),
    };

    let mut stmt = tx.prepare("SELECT key, value FROM response_headers WHERE packet_id = ?1;")?;
    let mut rows = stmt.query([packet_id])?;

    let mut response_headers = HashMap::default();
    while let Some(row) = rows.next()? {
        response_headers.insert(row.get_unwrap(0), row.get_unwrap(1));
    }

    let mut stmt = tx.prepare_cached("SELECT status, body FROM responses WHERE packet_id = ?1;")?;
    let mut rows = stmt.query([packet_id])?;

    let maybe_response = rows.next()?.map(|row| {
        (
            row.get_unwrap(0),
            HttpPacketContent {
                body: row.get_unwrap(1),
                headers: response_headers,
            },
        )
    });

    let mut log_row = HttpLogRow {
        metadata,
        request: HttpPacketContent {
            body: request_body,
            headers: request_headers,
        },
        response: None,
    };

    if let Some((status, response)) = maybe_response {
        let _ = log_row.metadata.status.insert(status);
        let _ = log_row.response.insert(response);
    }

    Ok(Some(log_row))
}
