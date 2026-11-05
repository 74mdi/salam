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

## GitHub Pages

The `deploy GitHub Pages` workflow builds and deploys this site on pushes to
`main`, or when run manually from the Actions tab. It configures the Next.js
static export for the repository's project-site path (for example,
`/salam/`) and attempts to enable GitHub Actions as the Pages source. If
repository policy prevents automatic enablement, an administrator must set
**Pages → Build and deployment → Source** to **GitHub Actions**.
