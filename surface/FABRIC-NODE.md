# Run a full Asolaria fabric node

`scripts/install.sh` gives you the **OS surface**. This gives you the **full node** — the OS *plus* the daemon engines running under your own key, the way a live Asolaria participant runs:

- **recall** (`:4796`) — your local inverted-index vault (HILBRA-IDX). Your corpus starts **empty** and is **never published**; public level-0 search is provably PII-free, everything above needs *your* key.
- **the 8-byte host** (`:5088`) — the Rust kernel with content-addressed **stubbed rooms** that spin up on demand.
- **the OS front-end** (`:4600`) — auto-detects both and lights their tiles.

## One command

```sh
git clone https://github.com/JesseBrown1980/asolaria-asi-os
cd asolaria-asi-os
sh scripts/install-full-node.sh      # Windows:  powershell -ExecutionPolicy Bypass -File scripts\install-full-node.ps1
```

It (1) mints your local key + seat, (2) clones + builds the public engines from **[asolaria-federation-1024](https://github.com/JesseBrown1980/asolaria-federation-1024)**, (3) builds the OS, (4) starts recall + the 8-byte host under your identity, and (5) opens the OS at `http://127.0.0.1:4600`. The `RECALL` and `KERNEL` tiles go green.

## What runs, and where your data lives

| piece | port | source | your data |
|---|---|---|---|
| OS front-end | 4600 | this repo | — |
| recall engine | 4796 | asolaria-federation-1024 | `~/.asolaria/recall/` (empty at first, yours) |
| 8-byte host | 5088 | asolaria-federation-1024 | in-memory rooms |
| key + seat | — | `scripts/keygen` | `~/.asolaria/recall.key`, `seat.*` |

**Your key and your corpus never leave your machine.** The engines are public; your data is not.

## Joining the wider fabric (Hilbra)

A node runs fully on its own. To cross-search with other nodes you speak **Hilbra** — the shared-key HBI/HBP protocol (keys stay off the wire, HMAC per request). The protocol + join handshake are specified in **[Hilbra](https://github.com/JesseBrown1980/Hilbra)** (`SPEC/join-handshake.md`, `SPEC/federation-protocol.md`). Level-0 is open + provably PII-free; everything above level-0 needs a shared key you exchange out-of-band.

## The engines (all public)

- **[asolaria-federation-1024](https://github.com/JesseBrown1980/asolaria-federation-1024)** — recall-serve, host8-serve, the kernel, agent-runtime, stubbed rooms (17 crates).
- **[Hilbra](https://github.com/JesseBrown1980/Hilbra)** — the Asolaria-internet protocol: spec + governance + secret-scan CI.
- **[asolaria-behcs-256](https://github.com/JesseBrown1980/asolaria-behcs-256)** — the BEHCS glyph substrate.

Nothing here fires on its own. **E = 0.**
