use crate::proxy::ProxyId;
use rusqlite::Connection;
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

const CONFIG_DIR: &str = ".yatangaki";
const CONFIG_FILE: &str = "config.db";

pub struct ProxyConfig {
    pub port: u16,
    pub id: ProxyId,
    pub auto_start: bool,
}

fn db_conn() -> Result<Connection, Box<dyn Error>> {
    let mut full_config_dir_buf: PathBuf =
        [&env::var("HOME").unwrap(), CONFIG_DIR].iter().collect();

    let full_config_dir = full_config_dir_buf.as_path();
    if !full_config_dir.exists() {
        fs::create_dir_all(full_config_dir)?;
    }
    full_config_dir_buf.push(CONFIG_FILE);
    Ok(Connection::open(
        full_config_dir_buf.as_path().to_str().unwrap(),
    )?)
}

pub fn init_config() -> Result<(), Box<dyn Error>> {
    let conn = db_conn()?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS proxies (
            proxy_id INTEGER UNIQUE NOT NULL,
            port INTEGER NOT NULL,
            auto_start INTEGER NOT NULL
        )",
        (),
    )?;

    Ok(())
}

pub fn load_proxies() -> Result<Vec<ProxyConfig>, Box<dyn Error>> {
    let conn = db_conn()?;
    let mut proxies = vec![];
    let mut stmt = conn.prepare("SELECT proxy_id, port, auto_start FROM proxies")?;
    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        proxies.push(ProxyConfig {
            id: row.get_unwrap(0),
            port: row.get_unwrap(1),
            auto_start: row.get_unwrap(2),
        });
    }

    Ok(proxies)
}

pub fn save_proxy(proxy: &ProxyConfig) -> Result<(), Box<dyn Error>> {
    let conn = db_conn()?;
    let mut stmt =
        conn.prepare("INSERT INTO proxies (proxy_id, port, auto_start) VALUES (?1, ?2, ?3)")?;

    stmt.execute((
        proxy.id,
        proxy.port as usize,
        if proxy.auto_start { 1 } else { 0 },
    ))?;

    Ok(())
}
