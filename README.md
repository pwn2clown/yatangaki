Client side scanning toolkit to get duplicates/informational in bug bounty.

Currently supports:

- URLSearchParams & Storage hooking
- JS Vanilla sinks hooking (innerHTML, document.write, etc)
- Canary injection in hash & query (location.* can't be hooked :c)

Planning to do:

- param injection (from detected params via hooking)
- prototype pollution detection
- jQuery, React, Vue sinks
- postMessage support?