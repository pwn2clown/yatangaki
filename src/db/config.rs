use crate::proxy::types::ProxyId;
use rusqlite::Connection;
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

static CONFIG_DB: LazyLock<Mutex<Connection>> =
    LazyLock::new(|| Mutex::new(init_db_conn().unwrap()));

pub struct ProxyConfig {
    pub port: u16,
    pub id: ProxyId,
    pub auto_start: bool,
}

fn init_db_conn() -> Result<Connection, Box<dyn Error>> {
    let mut full_config_dir_buf: PathBuf = [&env::var("HOME").unwrap(), super::CONFIG_DIR]
        .iter()
        .collect();

    let full_config_dir = full_config_dir_buf.as_path();
    if !full_config_dir.exists() {
        fs::create_dir_all(full_config_dir)?;
    }
    full_config_dir_buf.push("config.db");
    Ok(Connection::open(
        full_config_dir_buf.as_path().to_str().unwrap(),
    )?)
}

pub fn init_config() -> Result<(), Box<dyn Error>> {
    CONFIG_DB
        .lock()
        .unwrap()
        .execute_batch(&fs::read_to_string("./schema/config.sql")?)?;
    Ok(())
}

pub fn project_list() -> Result<Vec<String>, Box<dyn Error>> {
    let mut project_names = vec![];
    let conn = CONFIG_DB.lock().unwrap();
    let mut stmt = conn.prepare_cached("SELECT name FROM projects")?;
    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        project_names.push(row.get_unwrap(0));
    }

    Ok(project_names)
}

pub fn create_project(name: &str) -> Result<(), Box<dyn Error>> {
    CONFIG_DB
        .lock()
        .unwrap()
        .execute("INSERT INTO projects (name) VALUES(?1)", [name])?;
    Ok(())
}

pub fn delete_project(name: &str) -> Result<(), Box<dyn Error>> {
    CONFIG_DB
        .lock()
        .unwrap()
        .execute("DELETE FROM projects WHERE name = ?1", [name])?;
    fs::remove_dir_all(format!("{}/{}/{name}", env!("HOME"), super::CONFIG_DIR))?;
    Ok(())
}

pub fn load_proxies() -> Result<Vec<ProxyConfig>, Box<dyn Error>> {
    let conn = CONFIG_DB.lock().unwrap();
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
    let conn = CONFIG_DB.lock().unwrap();
    let mut stmt =
        conn.prepare("INSERT INTO proxies (proxy_id, port, auto_start) VALUES (?1, ?2, ?3)")?;

    stmt.execute((
        proxy.id,
        proxy.port as usize,
        if proxy.auto_start { 1 } else { 0 },
    ))?;

    Ok(())
}

/*
pub fn delete_proxy(proxy_id: ProxyId) -> Result<(), Box<dyn Error>> {
    Ok(())
}
*/
