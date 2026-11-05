const FEATURES = [
  {
    title: "End-to-end ready",
    body: "Sealed envelope protocol keeps message bodies opaque to the server.",
    icon: (
      <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8">
        <path d="M12 2l7 4v6c0 4.5-3 8-7 10-4-2-7-5.5-7-10V6l7-4z" />
        <path d="M9 12l2 2 4-4" />
      </svg>
    ),
  },
  {
    title: "SQLite only",
    body: "WAL mode, zero external services. Backup = copy one file.",
    icon: (
      <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8">
        <ellipse cx="12" cy="5" rx="8" ry="3" />
        <path d="M4 5v6c0 1.7 3.6 3 8 3s8-1.3 8-3V5" />
        <path d="M4 11v6c0 1.7 3.6 3 8 3s8-1.3 8-3v-6" />
      </svg>
    ),
  },
  {
    title: "Tiny footprint",
    body: "512 live sockets on a 1 vCPU / 2 GB box. Startup RSS under 40 MB.",
    icon: (
      <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8">
        <rect x="7" y="7" width="10" height="10" rx="2" />
        <path d="M12 2v3M12 19v3M2 12h3M19 12h3M5 5l2 2M17 17l2 2M19 5l-2 2M7 17l-2 2" />
      </svg>
    ),
  },
  {
    title: "Liquid glass",
    body: "Native Rust UI with frosted panels and springy motion. No Electron.",
    icon: (
      <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8">
        <path d="M12 3l1.8 5.2L19 10l-5.2 1.8L12 17l-1.8-5.2L5 10l5.2-1.8L12 3z" />
        <path d="M19 15l.9 2.6L22.5 18.5l-2.6.9L19 22l-.9-2.6-2.6-.9 2.6-.9L19 15z" />
      </svg>
    ),
  },
];

export default function Home() {
  return (
    <main>
      <nav className="nav">
        <div className="logo">
          <span className="k">K</span>
          <span className="word">koko</span>
        </div>
        <div className="nav-links">
          <a href="https://github.com/74mdi/salam/blob/main/docs/SELFHOSTING.md">Docs</a>
          <a href="https://github.com/74mdi/salam/blob/main/docs/SELFHOSTING.md">Self-hosting</a>
          <a href="https://github.com/74mdi/salam">GitHub</a>
          <span className="version">v0.0.1</span>
        </div>
      </nav>

      <section className="hero">
        <h1>Privacy that stays yours.</h1>
        <p>
          Self-hosted chat that runs on one binary and one SQLite file.
          Your server, your data, your rules.
        </p>
        <div className="cta-row">
          <a
            className="cta primary"
            href="https://github.com/74mdi/salam/blob/main/docs/SELFHOSTING.md"
          >
            Self-host in 60 seconds
          </a>
          <a className="cta secondary" href="https://github.com/74mdi/salam">
            ★ Star on GitHub
          </a>
        </div>
      </section>

      <section className="features">
        {FEATURES.map((f) => (
          <div key={f.title} className="glass feature">
            <div className="icon">{f.icon}</div>
            <h3>{f.title}</h3>
            <p>{f.body}</p>
          </div>
        ))}
      </section>

      <section className="code-card">
        <div className="glass inner">
          <div className="term-header">
            <span className="dot" />
            <span className="dot" />
            <span className="dot" />
            <span className="term-title">bash</span>
          </div>
          <pre>
            <span className="prompt">$</span> curl -fsSL get.koko.chat | sh{"\n"}
            <span className="prompt">$</span> koko-server --localhost{"\n"}
            <span className="comment"># listening on 127.0.0.1:8080 · db: koko.db</span>
          </pre>
        </div>
      </section>

      <footer className="footer">
        MIT licensed · built with Rust · koko 2026
      </footer>
    </main>
  );
}
