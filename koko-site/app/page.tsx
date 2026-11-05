const CHATS = [
  { initials: "mi", name: "mira", preview: "sent you the deploy notes", time: "9:41", active: true },
  { initials: "sh", name: "self-hosting crew", preview: "kai: db migration done", time: "9:12" },
  { initials: "ka", name: "kai", preview: "Typing…", time: "8:58" },
];

const FEATURES = [
  {
    number: "01",
    title: "Native on Linux",
    body: "A fast, focused desktop client built with Rust and iced. No Electron, no browser tab.",
  },
  {
    number: "02",
    title: "Your server. Your data.",
    body: "Run one small server binary. Your conversations live in a SQLite file you control.",
  },
  {
    number: "03",
    title: "Ready when you are",
    body: "Reconnects, syncs, and keeps a local conversation mirror, so your chat feels at home.",
  },
];

function Logo() {
  return (
    <a className="brand" href="#" aria-label="Koko home">
      <span className="brand-mark" aria-hidden="true">K</span>
      <span>koko</span>
    </a>
  );
}

function ClientPreview() {
  return (
    <figure className="preview-wrap" aria-label="Preview of the Koko Linux chat client">
      <div className="client-window">
        <aside className="client-sidebar">
          <div className="client-brand">
            <span className="brand-mark small" aria-hidden="true">K</span>
            <strong>koko</strong>
            <span className="search-icon" aria-hidden="true">⌕</span>
          </div>
          <div className="search-field"><span aria-hidden="true">⌕</span> Search conversations</div>
          <div className="chat-list">
            {CHATS.map((chat) => (
              <div className={`chat-item${chat.active ? " selected" : ""}`} key={chat.name}>
                <span className={`avatar avatar-${chat.initials}`}>{chat.initials}</span>
                <span className="chat-copy">
                  <span className="chat-meta"><strong>{chat.name}</strong><time>{chat.time}</time></span>
                  <span className={chat.preview === "Typing…" ? "typing" : "chat-preview"}>
                    {chat.preview}
                  </span>
                </span>
              </div>
            ))}
          </div>
          <div className="server-state"><span /> Connected to your server</div>
        </aside>

        <section className="client-conversation" aria-label="Conversation with mira">
          <header className="conversation-head">
            <span className="avatar avatar-mi">mi</span>
            <span className="peer-name"><strong>mira</strong><small><span /> online</small></span>
            <span className="window-actions" aria-hidden="true">···</span>
          </header>
          <div className="messages">
            <span className="day-label">Today</span>
            <p className="bubble received">hey! did the backup script finish?</p>
            <p className="bubble received">also — do we expose /healthz behind the proxy?</p>
            <p className="bubble sent">yep, sqlite .backup ran at 3am — koko.db is 41 MB</p>
            <p className="bubble received last">perfect. that&apos;s the last box on the self-host checklist 🎉</p>
          </div>
          <div className="composer"><span>Message mira</span><span className="send-button" aria-hidden="true">↑</span></div>
        </section>
      </div>
      <figcaption>One calm place to talk, running on your own server.</figcaption>
    </figure>
  );
}

export default function Home() {
  return (
    <main className="page">
      <nav className="nav shell" aria-label="Main navigation">
        <Logo />
        <div className="nav-links">
          <a href="https://github.com/74mdi/salam/blob/main/docs/SELFHOSTING.md">Docs</a>
          <a href="https://github.com/74mdi/salam">Source</a>
          <a className="nav-download" href="https://github.com/74mdi/salam/releases/latest">
            Linux client <span aria-hidden="true">↗</span>
          </a>
        </div>
      </nav>

      <section className="hero shell">
        <div className="hero-copy">
          <p className="eyebrow"><span /> PRIVATE CHAT, BUILT WITH RUST</p>
          <h1>Your conversations.<br /><span>Your machine.</span></h1>
          <p className="hero-description">
            A native Linux chat client for people who want a little more ownership.
            Run your own server. Keep your data close.
          </p>
          <div className="hero-actions">
            <a className="button button-primary" href="https://github.com/74mdi/salam/releases/latest">
              Get the Linux client <span aria-hidden="true">↗</span>
            </a>
            <a className="button button-quiet" href="https://github.com/74mdi/salam/blob/main/docs/SELFHOSTING.md">
              Set up your server
            </a>
          </div>
          <div className="hero-notes">
            <span>One server binary</span><i /><span>One SQLite file</span><i /><span>No Electron</span>
          </div>
        </div>
        <ClientPreview />
      </section>

      <section className="features shell" aria-labelledby="features-title">
        <div className="section-heading">
          <p className="eyebrow">A SMALLER, CALMER STACK</p>
          <h2 id="features-title">Chat, without the baggage.</h2>
        </div>
        <div className="feature-grid">
          {FEATURES.map((feature) => (
            <article className="feature" key={feature.number}>
              <span className="feature-number">{feature.number}</span>
              <h3>{feature.title}</h3>
              <p>{feature.body}</p>
            </article>
          ))}
        </div>
      </section>

      <section className="quickstart shell" id="get-started" aria-labelledby="quickstart-title">
        <div className="quickstart-copy">
          <p className="eyebrow">UP AND RUNNING IN MINUTES</p>
          <h2 id="quickstart-title">Your server.<br />Your rules.</h2>
          <p>Start a local server, open the client, and create your account. Move to your own host whenever you’re ready.</p>
          <a href="https://github.com/74mdi/salam/blob/main/docs/SELFHOSTING.md">
            Read the self-hosting guide <span aria-hidden="true">→</span>
          </a>
        </div>
        <div className="terminal" aria-label="Quick start commands">
          <div className="terminal-head"><span /><span /><span /><span>terminal</span></div>
          <pre><code><span className="terminal-muted"># Clone koko</span>{"\n"}git clone https://github.com/74mdi/salam{"\n"}cd salam{"\n\n"}<span className="terminal-muted"># Start a local server</span>{"\n"}cargo run -p koko-server -- --localhost</code></pre>
        </div>
      </section>

      <footer className="footer shell">
        <Logo />
        <span>Open source · Built with Rust · Made for your own server</span>
        <a href="https://github.com/74mdi/salam">GitHub <span aria-hidden="true">↗</span></a>
      </footer>
    </main>
  );
}
