"use client";

import Link from "next/link";
import { FormEvent, KeyboardEvent, useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  decodeMessageText,
  decodeServerFrame,
  encodeHello,
  encodeMessage,
  encodeTextMessage,
  encodeTyping,
  Envelope,
  MessageKind,
} from "./protocol";
import "./web.css";

type User = {
  id: number;
  handle: string;
  display_name: string;
  bio?: string;
  avatar_media_id?: string | null;
};

type Chat = {
  id: number;
  kind: "direct" | "group";
  title: string | null;
  members: string[];
  member_count: number;
  last_activity: number;
  unread: number;
};

type Message = Envelope & { localId?: string; pending?: boolean };
type UserHit = Pick<User, "handle" | "display_name" | "bio">;
type Preferences = {
  compact: boolean;
  timestamps: boolean;
  highContrast: boolean;
  reduceMotion: boolean;
};

const PREFS_KEY = "koko.web.preferences";
const DEFAULT_PREFS: Preferences = {
  compact: false,
  timestamps: true,
  highContrast: false,
  reduceMotion: false,
};
const textEncoder = new TextEncoder();

function normalizeServer(value: string) {
  return value.trim().replace(/\/+$/, "");
}

function getErrorMessage(body: unknown, fallback: string) {
  if (body && typeof body === "object" && "error" in body) {
    const error = (body as { error?: { message?: unknown } }).error;
    if (typeof error?.message === "string") return error.message;
  }
  return fallback;
}

function socketAddress(server: string) {
  const url = new URL(server);
  url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
  url.pathname = `${url.pathname.replace(/\/+$/, "")}/ws`;
  return url.toString();
}

function chatTitle(chat: Chat, currentUser: User | null) {
  if (chat.title) return chat.title;
  const otherMembers = chat.members.filter((member) => member !== currentUser?.handle);
  return otherMembers.join(", ") || "Notes to self";
}

function messageText(message: Message) {
  if (message.kind === 5) return decodeMessageText(message.body);
  if (message.kind === 0) {
    const raw = decodeMessageText(message.body);
    try {
      const payload = JSON.parse(raw) as { text?: unknown };
      if (typeof payload.text === "string") return payload.text;
    } catch {
      return raw;
    }
    return raw;
  }
  try {
    const media = JSON.parse(decodeMessageText(message.body)) as { filename?: string };
    return media.filename || "Attachment";
  } catch {
    return "Attachment";
  }
}

function MessageBody({ message, server, token }: { message: Message; server: string; token: string }) {
  const [url, setUrl] = useState("");
  const [loadError, setLoadError] = useState("");
  let media: { id: string; filename?: string; mime?: string } | null = null;
  const expectsMedia = [1, 2, 3, 4].includes(message.kind);
  if ([1, 2, 3, 4].includes(message.kind)) {
    try {
      media = JSON.parse(decodeMessageText(message.body));
    } catch {
      media = null;
    }
  }
  const mediaId = media?.id;

  useEffect(() => {
    if (!mediaId) return;
    let active = true;
    let objectUrl = "";
    fetch(`${normalizeServer(server)}/api/media/${encodeURIComponent(mediaId)}`, {
      headers: { Authorization: `Bearer ${token}` },
    })
      .then(async (response) => {
        if (!response.ok) throw new Error(`Attachment could not be loaded (HTTP ${response.status}).`);
        return response.blob();
      })
      .then((blob) => {
        objectUrl = URL.createObjectURL(blob);
        if (active) setUrl(objectUrl);
        else URL.revokeObjectURL(objectUrl);
      })
      .catch((cause: unknown) => {
        if (active) setLoadError(cause instanceof Error ? cause.message : "Attachment could not be loaded.");
      });
    return () => {
      active = false;
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [mediaId, server, token]);

  if (expectsMedia && !media) return <p>Invalid attachment.</p>;
  if (!media && message.kind === 0) {
    const raw = decodeMessageText(message.body);
    try {
      const payload = JSON.parse(raw) as { text?: unknown; reply?: { sender?: unknown; quote?: unknown } };
      if (typeof payload.text === "string" && payload.reply) {
        return <div className="web-reply-content">
          <div className="web-reply-quote"><strong>Reply to @{String(payload.reply.sender || "user")}</strong><span>{String(payload.reply.quote || "")}</span></div>
          <p>{payload.text}</p>
        </div>;
      }
    } catch {
      return <p>{raw}</p>;
    }
  }
  if (!media) return <p>{messageText(message)}</p>;
  if (loadError) return <p>{media.filename || "Attachment"} · {loadError}</p>;
  if (!url) return <p>{media.filename || "Loading attachment…"}</p>;
  if (message.kind === 1) return <a className="web-media-link" href={url} target="_blank" rel="noreferrer"><img className="web-image-preview" src={url} alt={media.filename || "Image attachment"} /></a>;
  if (message.kind === 3) return <video className="web-video-preview" src={url} controls preload="metadata" />;
  if (message.kind === 2) return <audio className="web-audio-preview" src={url} controls preload="metadata" />;
  return <a className="web-file-link" href={url} download={media.filename}>{media.filename || "Download attachment"}</a>;
}

function AvatarImage({
  mediaId,
  server,
  token,
  fallback,
}: {
  mediaId: string | null;
  server: string;
  token: string;
  fallback: string;
}) {
  const [url, setUrl] = useState("");

  useEffect(() => {
    if (!mediaId) {
      setUrl("");
      return;
    }
    let active = true;
    let objectUrl = "";
    fetch(`${normalizeServer(server)}/api/media/${encodeURIComponent(mediaId)}`, {
      headers: { Authorization: `****** },
    })
      .then(async (response) => {
        if (!response.ok) throw new Error("Avatar could not be loaded.");
        return response.blob();
      })
      .then((blob) => {
        objectUrl = URL.createObjectURL(blob);
        if (active) setUrl(objectUrl);
        else URL.revokeObjectURL(objectUrl);
      })
      .catch(() => {
        if (active) setUrl("");
      });
    return () => {
      active = false;
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [mediaId, server, token]);

  return url ? <img className="web-avatar-image" src={url} alt="" /> : <span>{fallback}</span>;
}

function formatTime(seconds: number) {
  return new Date(seconds * 1000).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

async function request<T>(
  server: string,
  path: string,
  token?: string,
  init: RequestInit = {},
): Promise<T> {
  let response: Response;
  try {
    response = await fetch(`${normalizeServer(server)}${path}`, {
      ...init,
      headers: {
        ...(init.body instanceof Blob ? {} : { "Content-Type": "application/json" }),
        ...(token ? { Authorization: `Bearer ${token}` } : {}),
        ...init.headers,
      },
    });
  } catch {
    throw new Error("Could not reach the server. Check its address and CORS settings.");
  }
  const text = await response.text();
  let value: unknown = null;
  if (text) {
    try {
      value = JSON.parse(text);
    } catch {
      value = text;
    }
  }
  if (!response.ok) {
    throw new Error(getErrorMessage(value, `Request failed (HTTP ${response.status}).`));
  }
  return value as T;
}

function ToolbarButton({
  label,
  active,
  onClick,
  children,
}: {
  label: string;
  active?: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button className={`web-rail-button${active ? " active" : ""}`} title={label} aria-label={label} onClick={onClick}>
      {children}
    </button>
  );
}

export default function WebClient() {
  const defaultServer = process.env.NEXT_PUBLIC_KOKO_SERVER || "http://127.0.0.1:8080";
  const [server, setServer] = useState(defaultServer);
  const [serverDraft, setServerDraft] = useState(defaultServer);
  const [token, setToken] = useState<string | null>(null);
  const [user, setUser] = useState<User | null>(null);
  const [chats, setChats] = useState<Chat[]>([]);
  const [activeChat, setActiveChat] = useState<number | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [replyingTo, setReplyingTo] = useState<Message | null>(null);
  const [forwarding, setForwarding] = useState<Message | null>(null);
  const [forwardTarget, setForwardTarget] = useState("");
  const [screen, setScreen] = useState<"chats" | "profile" | "appearance" | "account" | "ai">("chats");
  const [connected, setConnected] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [mode, setMode] = useState<"login" | "register">("login");
  const [handle, setHandle] = useState("");
  const [password, setPassword] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [bio, setBio] = useState("");
  const [avatarMediaId, setAvatarMediaId] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  const [chatQuery, setChatQuery] = useState("");
  const [chatFilter, setChatFilter] = useState<"all" | "unread" | "groups">("all");
  const [newHandle, setNewHandle] = useState("");
  const [groupTitle, setGroupTitle] = useState("");
  const [groupMembers, setGroupMembers] = useState("");
  const [addMember, setAddMember] = useState("");
  const [userQuery, setUserQuery] = useState("");
  const [userResults, setUserResults] = useState<UserHit[]>([]);
  const [typingHandle, setTypingHandle] = useState("");
  const [preferences, setPreferences] = useState<Preferences>(DEFAULT_PREFS);
  const [aiBot, setAiBot] = useState("");
  const socketRef = useRef<WebSocket | null>(null);
  const pendingFramesRef = useRef<Map<string, Uint8Array>>(new Map());
  const cursorRef = useRef(0);
  const typingTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const activeChatRef = useRef<number | null>(null);

  useEffect(() => {
    const savedServer = localStorage.getItem("koko.server");
    const savedToken = sessionStorage.getItem("koko.token");
    const savedPrefs = localStorage.getItem(PREFS_KEY);
    if (savedServer) {
      setServer(savedServer);
      setServerDraft(savedServer);
    }
    if (savedPrefs) {
      try {
        setPreferences({ ...DEFAULT_PREFS, ...JSON.parse(savedPrefs) });
      } catch {
        localStorage.removeItem(PREFS_KEY);
      }
    }
    setAiBot(localStorage.getItem("koko.ai.bot") || "");
    if (savedToken) setToken(savedToken);
  }, []);

  useEffect(() => {
    activeChatRef.current = activeChat;
  }, [activeChat]);

  const loadChats = useCallback(async (base = server, auth = token) => {
    if (!auth) return [];
    const rows = await request<Chat[]>(base, "/api/chats", auth);
    setChats(rows);
    return rows;
  }, [server, token]);

  const enterSession = useCallback(async (base: string, auth: string, account?: User) => {
    const normalized = normalizeServer(base);
    const currentUser = account || await request<User>(normalized, "/api/me", auth);
    cursorRef.current = 0;
    pendingFramesRef.current.clear();
    setServer(normalized);
    setServerDraft(normalized);
    setUser(currentUser);
    setToken(auth);
    sessionStorage.setItem("koko.token", auth);
    localStorage.setItem("koko.server", normalized);
    setDisplayName(currentUser.display_name || currentUser.handle);
    setBio(currentUser.bio || "");
    setAvatarMediaId(currentUser.avatar_media_id || null);
    setError("");
    const rows = await request<Chat[]>(normalized, "/api/chats", auth);
    setChats(rows);
    if (rows.length && activeChatRef.current === null) setActiveChat(rows[0].id);
  }, []);

  useEffect(() => {
    if (!token || user) return;
    let cancelled = false;
    request<User>(server, "/api/me", token)
      .then(async (account) => {
        if (cancelled) return;
        const rows = await request<Chat[]>(server, "/api/chats", token);
        if (cancelled) return;
        setUser(account);
        setDisplayName(account.display_name || account.handle);
        setBio(account.bio || "");
        setAvatarMediaId(account.avatar_media_id || null);
        setChats(rows);
        if (rows.length && activeChatRef.current === null) setActiveChat(rows[0].id);
      })
      .catch((cause: unknown) => {
        if (!cancelled) {
          sessionStorage.removeItem("koko.token");
          setToken(null);
          setError(cause instanceof Error ? cause.message : "Could not restore the session.");
        }
      });
    return () => { cancelled = true; };
  }, [server, token, user]);

  useEffect(() => {
    if (!token || !user) return;
    let stopped = false;
    let retryTimer: ReturnType<typeof setTimeout> | undefined;
    let socket: WebSocket | null = null;
    let delay = 500;
    const connect = () => {
      if (stopped) return;
      try {
        socket = new WebSocket(socketAddress(server));
        socket.binaryType = "arraybuffer";
        socketRef.current = socket;
        socket.onopen = () => {
          delay = 500;
          socket?.send(encodeHello(token, cursorRef.current || null));
          for (const frame of pendingFramesRef.current.values()) {
            if (socket?.readyState !== WebSocket.OPEN) break;
            socket.send(frame);
          }
        };
        socket.onmessage = (event) => {
          try {
            const frame = decodeServerFrame(new Uint8Array(event.data as ArrayBuffer));
            if (frame.type === "hello") {
              cursorRef.current = Math.max(cursorRef.current, frame.nextSeq - 1);
              setConnected(true);
            } else if (frame.type === "envelope") {
              const envelope = frame.envelope;
              cursorRef.current = Math.max(cursorRef.current, envelope.seq);
              if (activeChatRef.current === envelope.chat) {
                setMessages((items) => {
                  if (items.some((item) => item.seq === envelope.seq)) return items;
                  const pendingIndex = items.findIndex((item) =>
                    item.pending && item.sender === user?.id &&
                    decodeMessageText(item.body) === decodeMessageText(envelope.body));
                  if (pendingIndex < 0) return [...items, envelope];
                  const next = [...items];
                  next.splice(pendingIndex, 1, envelope);
                  return next;
                });
                void request(server, `/api/chats/${envelope.chat}/read`, token, {
                  method: "POST",
                  body: JSON.stringify({ up_to_seq: envelope.seq }),
                }).catch((cause: unknown) => setError(cause instanceof Error ? cause.message : "Could not mark messages read."));
              } else {
                setChats((items) => items.map((chat) => chat.id === envelope.chat ? { ...chat, unread: chat.unread + 1 } : chat));
              }
            } else if (frame.type === "typing") {
              setTypingHandle(frame.on && frame.chat === activeChatRef.current ? frame.handle : "");
            } else if (frame.type === "sent") {
              pendingFramesRef.current.delete(frame.id);
              setMessages((items) => items.map((item) =>
                item.localId === frame.id ? { ...item, seq: frame.seq, pending: false } : item));
            } else if (frame.type === "error") {
              setError(frame.message);
            }
          } catch (cause) {
            setError(cause instanceof Error ? cause.message : "Could not read the server message.");
          }
        };
        socket.onclose = () => {
          setConnected(false);
          if (!stopped) {
            retryTimer = setTimeout(connect, delay);
            delay = Math.min(delay * 2, 15_000);
          }
        };
        socket.onerror = () => socket?.close();
      } catch (cause) {
        setConnected(false);
        setError(cause instanceof Error ? cause.message : "Could not connect to the server.");
      }
    };
    connect();
    return () => {
      stopped = true;
      if (retryTimer) clearTimeout(retryTimer);
      socket?.close();
      socketRef.current = null;
      setConnected(false);
    };
  }, [server, token, user]);

  useEffect(() => {
    if (!token || activeChat === null) {
      setMessages([]);
      return;
    }
    let cancelled = false;
    request<Array<Omit<Message, "body"> & { body: number[] }>>(
      server,
      `/api/chats/${activeChat}/messages?after=0&limit=500`,
      token,
    )
      .then((rows) => {
        if (cancelled) return;
        const loaded = rows.map((row) => ({ ...row, body: Uint8Array.from(row.body) }));
        setMessages(loaded);
        const latest = loaded.reduce((seq, message) => Math.max(seq, message.seq), 0);
        if (latest) {
          cursorRef.current = Math.max(cursorRef.current, latest);
          void request(server, `/api/chats/${activeChat}/read`, token, {
            method: "POST",
            body: JSON.stringify({ up_to_seq: latest }),
          }).catch((cause: unknown) => setError(cause instanceof Error ? cause.message : "Could not mark messages read."));
        }
        setChats((items) => items.map((chat) => chat.id === activeChat ? { ...chat, unread: 0 } : chat));
      })
      .catch((cause: unknown) => setError(cause instanceof Error ? cause.message : "Could not load messages."));
    return () => { cancelled = true; };
  }, [activeChat, server, token]);

  useEffect(() => {
    const query = userQuery.trim().replace(/^@/, "");
    if (!token || query.length < 2) {
      setUserResults([]);
      return;
    }
    const timer = setTimeout(() => {
      request<UserHit[]>(server, `/api/users?q=${encodeURIComponent(query)}`, token)
        .then(setUserResults)
        .catch((cause: unknown) => setError(cause instanceof Error ? cause.message : "User search failed."));
    }, 180);
    return () => clearTimeout(timer);
  }, [server, token, userQuery]);

  const visibleChats = useMemo(() => chats.filter((chat) => {
    const title = chatTitle(chat, user).toLowerCase();
    const queryMatch = !chatQuery || title.includes(chatQuery.toLowerCase());
    const filterMatch = chatFilter === "all" || (chatFilter === "unread" ? chat.unread > 0 : chat.kind === "group");
    return queryMatch && filterMatch;
  }), [chatFilter, chatQuery, chats, user]);

  const active = chats.find((chat) => chat.id === activeChat) || null;

  async function submitAuth(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    setError("");
    const base = normalizeServer(serverDraft);
    try {
      const path = mode === "login" ? "/api/login" : "/api/register";
      const result = await request<{ token?: string; user?: User } | User>(base, path, undefined, {
        method: "POST",
        body: JSON.stringify(mode === "login"
          ? { handle: handle.trim().replace(/^@/, ""), password, device_name: "Koko Web" }
          : { handle: handle.trim().replace(/^@/, ""), password, display_name: displayName || handle }),
      });
      if (mode === "register") {
        setMode("login");
        setError("Account created. Sign in with your new account.");
        return;
      }
      const login = result as { token: string; user: User };
      await enterSession(base, login.token, login.user);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Authentication failed.");
    } finally {
      setBusy(false);
    }
  }

  async function createDirect(peer: string) {
    const normalized = peer.trim().replace(/^@/, "");
    if (!normalized || !token) return;
    try {
      const result = await request<{ chat_id: number }>(server, "/api/chats", token, {
        method: "POST",
        body: JSON.stringify({ kind: "direct", peer: normalized }),
      });
      await loadChats();
      setActiveChat(result.chat_id);
      setNewHandle("");
      setUserQuery("");
      setUserResults([]);
      setScreen("chats");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Could not open the conversation.");
    }
  }

  async function createGroup(event: FormEvent) {
    event.preventDefault();
    if (!token) return;
    try {
      const members = groupMembers.split(",").map((member) => member.trim().replace(/^@/, "")).filter(Boolean);
      const result = await request<{ chat_id: number }>(server, "/api/chats", token, {
        method: "POST",
        body: JSON.stringify({ kind: "group", title: groupTitle.trim(), members }),
      });
      await loadChats();
      setActiveChat(result.chat_id);
      setGroupTitle("");
      setGroupMembers("");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Could not create the group.");
    }
  }

  async function addGroupMember(event: FormEvent) {
    event.preventDefault();
    if (!token || activeChat === null || !addMember.trim()) return;
    try {
      await request(server, `/api/chats/${activeChat}/members`, token, {
        method: "POST",
        body: JSON.stringify({ handle: addMember.trim().replace(/^@/, "") }),
      });
      setAddMember("");
      await loadChats();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Could not add the group member.");
    }
  }

  async function openNotes() {
    if (!token) return;
    try {
      const result = await request<{ chat_id: number }>(server, "/api/chats/self", token, { method: "POST" });
      await loadChats();
      setActiveChat(result.chat_id);
      setScreen("chats");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Could not open notes to self.");
    }
  }

  async function sendMessage(event: FormEvent) {
    event.preventDefault();
    const text = draft.trim();
    if (!text || activeChat === null) return;
    const payload = replyingTo
      ? JSON.stringify({
          text,
          reply: {
            sender: replyingTo.sender_handle,
            quote: messageText(replyingTo).slice(0, 120),
          },
        })
      : text;
    const { bytes, id } = encodeTextMessage(activeChat, payload);
    const sent = sendFrame(id, bytes);
    setMessages((items) => [...items, {
      seq: -Date.now(),
      chat: activeChat,
      sender: user?.id || 0,
      sender_handle: user?.handle || "",
      kind: 0,
      body: textEncoder.encode(payload),
      created_at: Math.floor(Date.now() / 1000),
      localId: id,
      pending: true,
    }]);
    setDraft("");
    setReplyingTo(null);
    setError(sent ? "" : "Message queued until the connection returns.");
    setChats((items) => [...items].sort((a, b) => a.id === activeChat ? -1 : b.id === activeChat ? 1 : b.last_activity - a.last_activity));
  }

  async function sendForward(event: FormEvent) {
    event.preventDefault();
    if (!token || !forwarding || !forwardTarget.trim()) return;
    if (!socketRef.current || socketRef.current.readyState !== WebSocket.OPEN) {
      setError("Connection is offline. Reconnect before forwarding.");
      return;
    }
    try {
      const result = await request<{ chat_id: number }>(server, "/api/chats", token, {
        method: "POST",
        body: JSON.stringify({ kind: "direct", peer: forwardTarget.trim().replace(/^@/, "") }),
      });
      const frame = encodeMessage(result.chat_id, forwarding.kind, forwarding.body);
      sendFrame(frame.id, frame.bytes);
      setForwarding(null);
      setForwardTarget("");
      await loadChats();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Could not forward the message.");
    }
  }

  function sendFrame(id: string, frame: Uint8Array) {
    pendingFramesRef.current.set(id, frame);
    const socket = socketRef.current;
    if (socket?.readyState === WebSocket.OPEN) {
      try {
        socket.send(frame);
        return true;
      } catch {
        return false;
      }
    }
    return false;
  }

  function updateTyping(value: string) {
    setDraft(value);
    const socket = socketRef.current;
    if (activeChat === null || !socket || socket.readyState !== WebSocket.OPEN) return;
    socket.send(encodeTyping(activeChat, value.length > 0));
    if (typingTimer.current) clearTimeout(typingTimer.current);
    typingTimer.current = setTimeout(() => {
      const current = socketRef.current;
      if (current?.readyState === WebSocket.OPEN && activeChatRef.current !== null) {
        current.send(encodeTyping(activeChatRef.current, false));
      }
    }, 1800);
  }

  async function uploadFile(file: File) {
    if (!token || activeChat === null) return;
    if (!socketRef.current || socketRef.current.readyState !== WebSocket.OPEN) {
      setError("Connection is offline. Reconnect before uploading an attachment.");
      return;
    }
    try {
      const media = await request<{ id: string; filename: string; mime: string }>(
        server,
        `/api/media?name=${encodeURIComponent(file.name)}`,
        token,
        { method: "POST", body: file, headers: { "Content-Type": file.type || "application/octet-stream" } },
      );
      const kind: MessageKind = file.type.startsWith("image/") ? 1 : file.type.startsWith("video/") ? 3 : 4;
      const body = textEncoder.encode(JSON.stringify(media));
      const frame = encodeMessage(activeChat, kind, body);
      if (!sendFrame(frame.id, frame.bytes)) {
        setError("Attachment uploaded and queued until the connection returns.");
      }
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Could not upload the attachment.");
    }
  }

  async function saveProfile(event: FormEvent) {
    event.preventDefault();
    if (!token) return;
    try {
      const account = await request<User>(server, "/api/me", token, {
        method: "PUT",
        body: JSON.stringify({ display_name: displayName, bio, avatar_media_id: avatarMediaId }),
      });
      setUser(account);
      setError("Profile saved.");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Could not save your profile.");
    }
  }

  async function uploadAvatar(file: File) {
    if (!token) return;
    try {
      const media = await request<{ id: string }>(
        server,
        `/api/media?name=${encodeURIComponent(file.name)}`,
        token,
        { method: "POST", body: file, headers: { "Content-Type": file.type || "application/octet-stream" } },
      );
      setAvatarMediaId(media.id);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Could not upload the profile image.");
    }
  }

  async function copyMessage(message: Message) {
    try {
      await navigator.clipboard.writeText(messageText(message));
    } catch {
      setError("Could not copy the message. Check browser clipboard permissions.");
    }
  }

  async function logout() {
    if (token) {
      try {
        await request(server, "/api/logout", token, { method: "POST" });
      } catch (cause) {
        setError(cause instanceof Error ? cause.message : "Could not sign out cleanly.");
      }
    }
    sessionStorage.removeItem("koko.token");
    pendingFramesRef.current.clear();
    cursorRef.current = 0;
    setToken(null);
    setUser(null);
    setChats([]);
    setActiveChat(null);
    setMessages([]);
  }

  function savePreferences(next: Preferences) {
    setPreferences(next);
    localStorage.setItem(PREFS_KEY, JSON.stringify(next));
  }

  function handleComposerKey(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      event.currentTarget.form?.requestSubmit();
    }
  }

  if (!token || !user) {
    return (
      <main className="web-auth-screen">
        <form className="web-auth-card" onSubmit={submitAuth}>
          <Link className="web-brand" href="/"><span className="web-brand-mark">K</span><strong>koko</strong></Link>
          <p className="web-eyebrow">YOUR SERVER, YOUR CONVERSATIONS</p>
          <h1>{mode === "login" ? "Welcome back." : "Create your account."}</h1>
          <p className="web-muted">Connect to the same Koko server as your desktop client.</p>
          <label>Server address</label>
          <input className="web-input" type="url" required value={serverDraft} onChange={(event) => setServerDraft(event.target.value)} placeholder="https://chat.example.com" />
          <label>Username</label>
          <input className="web-input" required autoComplete="username" value={handle} onChange={(event) => setHandle(event.target.value)} placeholder="your handle" />
          {mode === "register" && <>
            <label>Display name</label>
            <input className="web-input" value={displayName} onChange={(event) => setDisplayName(event.target.value)} placeholder="How people see you" />
          </>}
          <label>Password</label>
          <input className="web-input" type="password" required minLength={8} autoComplete={mode === "login" ? "current-password" : "new-password"} value={password} onChange={(event) => setPassword(event.target.value)} />
          {error && <p className="web-notice">{error}</p>}
          <button className="web-primary-button" disabled={busy}>{busy ? "Connecting…" : mode === "login" ? "Sign in" : "Create account"}</button>
          <button className="web-switch-mode" type="button" onClick={() => { setMode(mode === "login" ? "register" : "login"); setError(""); }}>
            {mode === "login" ? "New to this server? Create an account" : "Already have an account? Sign in"}
          </button>
          <p className="web-footnote">Your password and session stay in this browser; Koko servers require HTTPS for secure public connections.</p>
        </form>
      </main>
    );
  }

  return (
    <main className={`web-app${preferences.highContrast ? " web-high-contrast" : ""}${preferences.reduceMotion ? " web-reduced-motion" : ""}${preferences.compact ? " web-compact" : ""}`}>
      <nav className="web-rail" aria-label="App navigation">
        <button className="web-profile-avatar" title="Profile" onClick={() => setScreen("profile")}>
          <AvatarImage mediaId={avatarMediaId} server={server} token={token} fallback={(user.display_name || user.handle).charAt(0).toUpperCase()} />
        </button>
        <span className="web-rail-divider" />
        <ToolbarButton label="Chats" active={screen === "chats"} onClick={() => setScreen("chats")}>⌂</ToolbarButton>
        <span className="web-rail-spacer" />
        <ToolbarButton label="Settings" active={screen !== "chats"} onClick={() => setScreen("profile")}>⚙</ToolbarButton>
      </nav>

      <aside className={`web-sidebar${screen !== "chats" ? " settings-sidebar" : ""}`}>
        <header className="web-sidebar-header">
          <Link href="/" className="web-brand"><span className="web-brand-mark">K</span><strong>koko</strong></Link>
          <span className="web-rail-spacer" />
          {screen === "chats" && <button className="web-icon-button" title="Settings" onClick={() => setScreen("profile")}>⚙</button>}
        </header>
        {screen === "chats" ? <>
          <div className="web-connection"><span className={connected ? "web-status online" : "web-status"} />{connected ? "Connected to your server" : "Connecting to server…"}</div>
          <input className="web-input web-chat-search" placeholder="Search conversations" value={chatQuery} onChange={(event) => setChatQuery(event.target.value)} />
          <div className="web-filters">
            {(["all", "unread", "groups"] as const).map((filter) => <button key={filter} className={chatFilter === filter ? "selected" : ""} onClick={() => setChatFilter(filter)}>{filter}</button>)}
          </div>
          <div className="web-chat-list">
            {visibleChats.map((chat) => <button key={chat.id} className={`web-chat-item${activeChat === chat.id ? " selected" : ""}`} onClick={() => setActiveChat(chat.id)}>
              <span className="web-chat-avatar">{chatTitle(chat, user).charAt(0).toUpperCase()}</span>
              <span className="web-chat-copy"><span className="web-chat-title"><strong>{chatTitle(chat, user)}</strong>{chat.unread > 0 && <b>{chat.unread}</b>}</span><small>{chat.kind === "group" ? `${chat.member_count} members` : "Direct message"}</small></span>
            </button>)}
            {!visibleChats.length && <p className="web-empty-hint">No conversations found.</p>}
          </div>
          <div className="web-create-tools">
            <form className="web-inline-form" onSubmit={(event) => { event.preventDefault(); void createDirect(newHandle); }}>
              <input className="web-input" value={newHandle} onChange={(event) => { setNewHandle(event.target.value); setUserQuery(event.target.value); }} placeholder="Start a direct chat" />
              <button className="web-icon-button" aria-label="Start direct chat">+</button>
            </form>
            {userResults.length > 0 && <div className="web-user-results">{userResults.map((result) => <button key={result.handle} onClick={() => void createDirect(result.handle)}><strong>{result.display_name || result.handle}</strong><span>@{result.handle}</span></button>)}</div>}
            <form className="web-group-form" onSubmit={createGroup}>
              <input className="web-input" value={groupTitle} onChange={(event) => setGroupTitle(event.target.value)} placeholder="New group name" />
              <input className="web-input" value={groupMembers} onChange={(event) => setGroupMembers(event.target.value)} placeholder="Members, comma separated" />
              <button className="web-secondary-button" disabled={!groupTitle.trim()}>Create group</button>
            </form>
          </div>
        </> : <>
          <p className="web-eyebrow web-settings-label">SETTINGS</p>
          {(["profile", "appearance", "account", "ai"] as const).map((item) => <button key={item} className={`web-settings-nav${screen === item ? " selected" : ""}`} onClick={() => setScreen(item)}>{item === "ai" ? "AI & bots" : item[0].toUpperCase() + item.slice(1)}</button>)}
        </>}
      </aside>

      <section className="web-main">
        {screen === "chats" && active ? <>
          <header className="web-conversation-header">
            <button className="web-mobile-back" onClick={() => setActiveChat(null)}>←</button>
            <span className="web-chat-avatar large">{chatTitle(active, user).charAt(0).toUpperCase()}</span>
            <div><strong>{chatTitle(active, user)}</strong><small>{typingHandle ? `@${typingHandle} is typing…` : connected ? "Connected" : "Reconnecting"}</small></div>
            {active.kind === "group" && <form className="web-add-member" onSubmit={addGroupMember}>
              <input className="web-input" value={addMember} onChange={(event) => setAddMember(event.target.value)} placeholder="Add member" />
              <button className="web-icon-button" title="Add member">+</button>
            </form>}
            <span className={`web-status${connected ? " online" : ""}`} />
          </header>
          <div className="web-messages">
            {messages.map((message) => <article key={message.localId || `${message.seq}-${message.sender}`} className={`web-message${message.sender === user.id ? " mine" : ""}`}>
              <span className="web-message-author">{message.sender === user.id ? "You" : `@${message.sender_handle}`}</span>
              <MessageBody message={message} server={server} token={token} />
              {preferences.timestamps && <time>{formatTime(message.created_at)}{message.pending ? " · sending" : ""}</time>}
              <div className="web-message-actions"><button title="Copy message" onClick={() => void copyMessage(message)}>Copy</button><button title="Reply" onClick={() => setReplyingTo(message)}>Reply</button><button title="Forward" onClick={() => { setForwarding(message); setForwardTarget(""); }}>Forward</button></div>
            </article>)}
            {!messages.length && <div className="web-empty-conversation"><span>✦</span><strong>This is the beginning.</strong><p>Send a message to get the conversation going.</p></div>}
          </div>
          {forwarding && <form className="web-forward-panel" onSubmit={sendForward}>
            <strong>Forward message</strong>
            <input className="web-input" value={forwardTarget} onChange={(event) => setForwardTarget(event.target.value)} placeholder="Username" />
            <button className="web-secondary-button" disabled={!forwardTarget.trim()}>Forward</button>
            <button className="web-icon-button" type="button" onClick={() => setForwarding(null)}>×</button>
          </form>}
          {replyingTo && <div className="web-reply-banner">
            <div><strong>Replying to @{replyingTo.sender_handle}</strong><span>{messageText(replyingTo)}</span></div>
            <button className="web-icon-button" onClick={() => setReplyingTo(null)}>×</button>
          </div>}
          <form className="web-composer" onSubmit={sendMessage}>
            <label className="web-attach" title="Attach a file">＋<input type="file" onChange={(event) => { const file = event.target.files?.[0]; if (file) void uploadFile(file); event.currentTarget.value = ""; }} /></label>
            <input className="web-input" value={draft} onChange={(event) => updateTyping(event.target.value)} onKeyDown={handleComposerKey} placeholder={`Message ${chatTitle(active, user)}`} />
            <button className="web-send-button" disabled={!draft.trim() || !connected} title="Send message">↑</button>
          </form>
        </> : screen === "chats" ? <div className="web-no-chat"><span>✦</span><h2>Select a conversation</h2><p>Choose a chat or start a new one from the sidebar.</p></div>
          : <SettingsPanel
            screen={screen}
            user={user}
            displayName={displayName}
            bio={bio}
            avatarMediaId={avatarMediaId}
            setDisplayName={setDisplayName}
            setBio={setBio}
            uploadAvatar={(file) => void uploadAvatar(file)}
            saveProfile={saveProfile}
            preferences={preferences}
            savePreferences={savePreferences}
            server={server}
            token={token}
            connected={connected}
            openNotes={() => void openNotes()}
            aiBot={aiBot}
            setAiBot={(value) => { setAiBot(value); localStorage.setItem("koko.ai.bot", value); }}
            onLogout={() => void logout()}
          />}
        {error && <button className="web-error-banner" onClick={() => setError("")}>{error}<span>×</span></button>}
      </section>
    </main>
  );
}

function SettingsPanel({
  screen,
  user,
  displayName,
  bio,
  avatarMediaId,
  setDisplayName,
  setBio,
  uploadAvatar,
  saveProfile,
  preferences,
  savePreferences,
  server,
  token,
  connected,
  openNotes,
  aiBot,
  setAiBot,
  onLogout,
}: {
  screen: "profile" | "appearance" | "account" | "ai";
  user: User;
  displayName: string;
  bio: string;
  avatarMediaId: string | null;
  setDisplayName: (value: string) => void;
  setBio: (value: string) => void;
  uploadAvatar: (file: File) => void;
  saveProfile: (event: FormEvent) => void;
  preferences: Preferences;
  savePreferences: (value: Preferences) => void;
  server: string;
  token: string;
  connected: boolean;
  openNotes: () => void;
  aiBot: string;
  setAiBot: (value: string) => void;
  onLogout: () => void;
}) {
  const title = { profile: "Profile", appearance: "Appearance", account: "Account & connection", ai: "AI & bots" }[screen];
  return <div className="web-settings-content">
    <header className="web-settings-heading"><p className="web-eyebrow">YOUR KOKO</p><h1>{title}</h1></header>
    {screen === "profile" && <form className="web-settings-card" onSubmit={saveProfile}>
      <h2>Public profile</h2><p>Your name and bio are shared with people on this server.</p>
      <label className="web-avatar-picker"><AvatarImage mediaId={avatarMediaId} server={server} token={token} fallback={(displayName || user.handle).charAt(0).toUpperCase()} /><span>Change profile image<input type="file" accept="image/*" onChange={(event) => { const file = event.target.files?.[0]; if (file) uploadAvatar(file); event.currentTarget.value = ""; }} /></span></label>
      <label>Display name<input className="web-input" value={displayName} onChange={(event) => setDisplayName(event.target.value)} /></label>
      <label>Bio<textarea className="web-input" maxLength={160} rows={4} value={bio} onChange={(event) => setBio(event.target.value)} /></label>
      <button className="web-primary-button">Save profile</button>
    </form>}
    {screen === "appearance" && <div className="web-settings-card">
      <h2>Reading & appearance</h2>
      <PreferenceRow label="Compact conversation list" checked={preferences.compact} onChange={(checked) => savePreferences({ ...preferences, compact: checked })} />
      <PreferenceRow label="Show message timestamps" checked={preferences.timestamps} onChange={(checked) => savePreferences({ ...preferences, timestamps: checked })} />
      <PreferenceRow label="High contrast borders" checked={preferences.highContrast} onChange={(checked) => savePreferences({ ...preferences, highContrast: checked })} />
      <PreferenceRow label="Reduce motion" checked={preferences.reduceMotion} onChange={(checked) => savePreferences({ ...preferences, reduceMotion: checked })} />
    </div>}
    {screen === "account" && <div className="web-settings-card">
      <h2>Connection</h2><p>Signed in as <strong>@{user.handle}</strong></p><p className="web-muted">{server}</p><p><span className={`web-status${connected ? " online" : ""}`} /> {connected ? "WebSocket connected" : "Reconnecting to server…"}</p>
      <button className="web-secondary-button" onClick={openNotes}>Open notes to self</button>
      <button className="web-secondary-button" onClick={() => window.location.reload()}>Reconnect</button>
      <button className="web-danger-button" onClick={onLogout}>Sign out</button>
    </div>}
    {screen === "ai" && <div className="web-settings-card">
      <h2>AI & bots</h2><p>Choose an account to mention in chats. Koko does not run AI actions from the browser; this preference is stored locally.</p>
      <label>Bot handle<input className="web-input" value={aiBot} onChange={(event) => setAiBot(event.target.value)} placeholder="@assistant" /></label>
      <p className="web-muted">Provider configuration and AI actions are not currently exposed by the Koko server.</p>
    </div>}
  </div>;
}

function PreferenceRow({ label, checked, onChange }: { label: string; checked: boolean; onChange: (value: boolean) => void }) {
  return <label className="web-preference-row"><span>{label}</span><input type="checkbox" checked={checked} onChange={(event) => onChange(event.target.checked)} /></label>;
}
