# CI / CD — GitHub Actions

Everything below lives in `.github/workflows/`. Push to `main` and the
whole matrix runs; tag `v*` and you get a release.

## Workflows

| Workflow | Trigger | What it does |
|---|---|---|
| `ci.yml` | push / PR | the full quality matrix (below) |
| `release.yml` | tag `v*` | builds server binaries (linux/windows/macos), native client with wgpu, the wasm bundle, and publishes a GitHub Release with checksums |
| `docker.yml` | push to main, tags | builds and pushes `ghcr.io/<owner>/<repo>` (semver + sha tags) |
| `deploy.yml` | manual (workflow_dispatch) | one-click deploy over SSH (see below) |

## The `ci.yml` matrix

| Job | Proves |
|---|---|
| server-stack | `cargo fmt --check`, `clippy -D warnings`, all 34 tests (incl. 8 end-to-end integration tests with real WS clients) |
| client | iced client builds + clippy-clean on Linux (tiny-skia renderer — no GPU needed in CI) |
| wasm | koko-web compiles for `wasm32-unknown-unknown` and `trunk build` produces the bundle |
| android-check | koko-client cross-compiles for `aarch64-linux-android` with the Android NDK |
| site | koko-site static-exports with `next build` |
| audit | `cargo audit --deny warnings` + `cargo deny` (license policy in `deny.toml`) |
| smoke | boots the real server binary, registers a user, logs in, hits `/api/me` and `/healthz` over HTTP |

## Making a release

```bash
git tag v0.0.1
git push origin v0.0.1
```

`release.yml` builds and attaches:

- `koko-server-{linux-amd64,windows-amd64,macos-arm64}` tarballs/zips
- `koko-client-*` (with the `wgpu` renderer feature)
- `koko-web.tar.gz` (static bundle)
- `.sha256` checksums for everything, generated release notes

The Docker image gets `v0.0.1`, `0.0`, and `sha-…` tags from the same
push (`docker.yml`).

## One-click deploy

`deploy.yml` is a template for shipping to your own box over SSH.

1. Copy the repo's `deploy/` files to the server once:

   ```bash
   # on your server
   sudo mkdir -p /opt/koko
   sudo cp deploy/docker-compose.yml /opt/koko/   # from a clone
   ```

2. Add repo secrets (Settings → Secrets and variables → Actions):

   | Secret | Value |
   |---|---|
   | `DEPLOY_HOST` | `koko.example.com` |
   | `DEPLOY_USER` | `deploy` (needs docker group) |
   | `DEPLOY_SSH_KEY` | the private key |

3. Actions tab → **deploy** → Run workflow → pick an image tag.

The job writes the image tag, pulls, restarts, and curls `/healthz` to
confirm the deployment is live.

## Caching

`Swatinem/rust-cache@v2` on every Rust job; Docker uses GHA layer caches
(`cache-from/to: type=gha`). A cold CI run is ~15 min; warm runs are
~4 min.

## Forking this for your own deployment

The workflows use `${{ github.repository }}` for the image name, so a fork
just works: GHCR images land under your own namespace, and `deploy.yml`
points at them. Nothing is hard-coded to koko-chat.
