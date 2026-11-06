# koko-site

Next.js landing page and browser chat client. `output: "export"` produces a
static site with no server runtime; the `/web/` client connects directly to a
Koko server over REST and the binary WebSocket protocol.

## Build

```bash
npm ci
npm run build        # → ./out (pure static files)
```

Set the Vercel project root directory to `koko-site`, or deploy `out/`
anywhere static. The client never stores chat data on Vercel.

## GitHub Pages

The `deploy GitHub Pages` workflow builds and deploys this site on pushes to
`main`, or when run manually from the Actions tab. It configures the Next.js
static export for the repository's project-site path (for example,
`/salam/`) and attempts to enable GitHub Actions as the Pages source. If
repository policy prevents automatic enablement, an administrator must set
**Pages → Build and deployment → Source** to **GitHub Actions**.
