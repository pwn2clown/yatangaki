CREATE TABLE IF NOT EXISTS proxies (
	proxy_id INTEGER UNIQUE NOT NULL,
        port INTEGER NOT NULL,
        auto_start INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS projects (
        name TEXT NOT NULL
);
