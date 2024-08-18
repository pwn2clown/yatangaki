CREATE TABLE IF NOT EXISTS requests (
        packet_id INTEGER UNIQUE NOT NULL,
        proxy_id INTEGER NOT NULL,
        method TEXT NOT NULL,
        authority TEXT NOT NULL,
        path TEXT NOT NULL,
        query TEXT,
        body BLOB
);
        
CREATE TABLE IF NOT EXISTS responses (
        packet_id INTEGER UNIQUE NOT NULL,
        status INTEGER NOT NULL,
        body BLOB
);

CREATE TABLE IF NOT EXISTS request_headers (
        packet_id INTEGER NOT NULL,
        key TEXT NOT NULL,
        value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS response_headers (
        packet_id INTEGER NOT NULL,
        key TEXT NOT NULL,
        value TEXT NOT NULL
);
