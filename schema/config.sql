CREATE TABLE IF NOT EXISTS proxies (
	proxy_id INTEGER UNIQUE NOT NULL,
        port INTEGER NOT NULL,
        auto_start INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS projects (
	project_id INTEGER UNIQUE NOT NULL,
        name TEXT NOT NULL
);
