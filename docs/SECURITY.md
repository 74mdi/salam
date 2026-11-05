# Security posture

Honest document. What v0.1 defends against, what it does not, and why
each tradeoff was made. If you find a hole, open an issue (or a private
advisory via GitHub security tab) — patches welcome.

## Threat model

Self-hosted, single-tenant, small-group chat. The operators we care about:

| Adversary | v0.1 defense |
|---|---|
| Someone with the server database file | Passwords are Argon2id hashes; tokens exist only as SHA-256 digests — nothing reusable leaks |
| Brute-force login / registration spam | Fixed-window rate limits (IP + handle keyed), login-lockout, timing-equalized failures |
| Account enumeration | Identical 401 + identical Argon2 burn for unknown users and wrong passwords |
| Malicious peer in a chat | Server enforces membership on every send and every read |
| Storage exhaustion (media bombs) | 25 MiB/file, 4 GiB/user, LRU eviction, per-socket and per-user rate budgets |
| Slow-loris style WS abuse | 10 s auth deadline, 512 connection cap, send budgets, frame size caps |

## What v0.1 does NOT defend against (yet)

- **The server operator reading your messages.** Bodies are opaque on the
  wire, but the shipped cipher is `NoopCipher` — the demo client sends
  clear text, and the server stores what it is given. Real end-to-end
  encryption (Signal protocol: X3DH + Double Ratchet, sender keys for
  groups) is the next milestone; it lands behind
  `koko_core::e2e::Cipher` without wire or schema changes.
- TLS termination is delegated to a reverse proxy (Caddy/nginx configs
  ship in `deploy/`). Run koko without one only in localhost mode.
- No 2FA/TOTP yet; no audit log; no per-device key revocation UI.

## Password hashing

Argon2id with `m = 32 MiB, t = 2, p = 1` — read the reasoning in
`koko-core/src/password.rs` before changing it. Summary: the spec box is
1 vCPU / 2 GB. These parameters keep a login-storm from OOMing the box
(~35 MB peak per hash) while staying in the tens of milliseconds on a weak
vCPU. The cost versus OWASP's 19 MiB+ floor is a modest reduction in GPU
cracking resistance — the right trade for a username-only, no-email,
self-hosted server where availability of a $4 box matters more than the
last 2 bits of hardness. On beefier hardware, raise `m` (stored hashes
embed their parameters, so old hashes keep verifying).

## Sessions

- 32 bytes from the OS CSPRNG, base64url (43 chars) on the wire.
- **Only `SHA-256(token)` is stored** — a stolen database has no usable
  credentials. Lookup is by digest; with 256-bit uniform tokens, timing
  side channels on the index probe are not exploitable. In-memory digest
  comparisons use constant-time equality (`subtle`).
- Device-bound, expiring (30 d default), revocable individually and all
  at once. Logout deletes the row.
- `cache-control: no-store` on token responses.

## Transport

- WS frames are binary and length-capped (1 MiB decode, 256 KiB body);
  malformed input is a typed `Error` frame, never a panic.
- JSON bodies are actix-extracted with strict types; unknown fields
  rejected, sizes bounded by the payload config.
- `unsafe` is forbidden workspace-wide (enforced by lint, not convention).
  `cargo audit` + `cargo deny` run in CI.

## Operational hardening

- The shipped systemd unit runs the server with `NoNewPrivileges`,
  `ProtectSystem=strict`, `PrivateTmp`, `PrivateDevices`,
  `MemoryDenyWriteExecute`, and a dedicated user.
- The Docker image is Debian-slim + curl healthcheck, state under one
  volume, no privilege.
- Registration defaults to closed on public binds; localhost mode is
  loopback-only.

## Reporting

Open a GitHub security advisory, or an issue marked `security`. We are
a small project — disclose responsibly and we will fix fast.
