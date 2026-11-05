# koko-site

Static **landing page only** — Next.js with `output: "export"`, no server
runtime, and **never part of the chat path**. (The chat web client is
`koko-web`, a Leptos/WASM app; this directory exists to explain koko, not
to run it.)

## Build

```bash
npm install
npm run build        # → ./out (pure static files)
```

Deploy `out/` anywhere static: GitHub Pages, any CDN, or the same reverse
proxy that fronts your koko-server.
