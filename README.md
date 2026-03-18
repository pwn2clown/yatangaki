# Yatangaki

Client side scanning toolkit to get duplicates/informational in bug bounty.

Currently supports:

## URL param guessing

- URLSearchParams hooking
- Canary injection in hash & query (location.* can't be hooked :c)
- param injection (from detected params via split("=") hooking)
- query param extracted from requests sent by page

## Sink hooking

- JS Vanilla sinks hooking (innerHTML, document.write, etc)

Planning to do:

- prototype pollution detection (P1)
- app crawling (P2)
- jQuery, React, Vue sinks (P3)
- postMessage support? (P4)