#!/usr/bin/bash

docker run -p 3000:3000 --rm --init -it --workdir /home/pwuser --user pwuser mcr.microsoft.com/playwright:v1.58.2-noble /bin/sh -c "npx -y playwright@1.58.2 run-server --port 3000 --host 0.0.0.0"
Listening on ws://0.0.0.0:3000/