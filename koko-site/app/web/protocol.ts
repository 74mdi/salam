const MAGIC = [0x4b, 0x4f, 0x4b, 0x31];
const textEncoder = new TextEncoder();
const textDecoder = new TextDecoder();

export type MessageKind = 0 | 1 | 2 | 3 | 4 | 5;

export type Envelope = {
  seq: number;
  chat: number;
  sender: number;
  sender_handle: string;
  kind: MessageKind;
  body: Uint8Array;
  created_at: number;
};

export type ServerFrame =
  | { type: "hello"; nextSeq: number }
  | { type: "envelope"; envelope: Envelope }
  | { type: "typing"; chat: number; handle: string; on: boolean }
  | { type: "sent"; id: string; seq: number }
  | { type: "error"; message: string }
  | { type: "pong" };

class Writer {
  private data: number[] = [];

  u8(value: number) {
    this.data.push(value & 0xff);
  }

  u32(value: number) {
    for (let i = 0; i < 4; i += 1) this.u8(value >>> (i * 8));
  }

  u64(value: number | bigint) {
    const bytes = new Uint8Array(8);
    new DataView(bytes.buffer).setBigUint64(0, BigInt(value), true);
    this.raw(bytes);
  }

  i64(value: number) {
    const bytes = new Uint8Array(8);
    new DataView(bytes.buffer).setBigInt64(0, BigInt(value), true);
    this.raw(bytes);
  }

  string(value: string) {
    const bytes = textEncoder.encode(value);
    this.u64(bytes.length);
    this.raw(bytes);
  }

  raw(bytes: Uint8Array) {
    for (const byte of bytes) this.data.push(byte);
  }

  finish() {
    return Uint8Array.from([...MAGIC, ...this.data]);
  }
}

class Reader {
  private offset = 4;
  private view: DataView;

  constructor(private bytes: Uint8Array) {
    if (bytes.length < 4 || MAGIC.some((byte, index) => bytes[index] !== byte)) {
      throw new Error("Unrecognized Koko protocol frame.");
    }
    this.view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  }

  private check(length: number) {
    if (length < 0 || this.offset + length > this.bytes.length) {
      throw new Error("Server sent an incomplete Koko protocol frame.");
    }
  }

  u8() {
    this.check(1);
    return this.view.getUint8(this.offset++);
  }

  u32() {
    this.check(4);
    const value = this.view.getUint32(this.offset, true);
    this.offset += 4;
    return value;
  }

  u64() {
    this.check(8);
    const value = this.view.getBigUint64(this.offset, true);
    this.offset += 8;
    const number = Number(value);
    if (!Number.isSafeInteger(number)) {
      throw new Error("Server value exceeds JavaScript's safe integer range.");
    }
    return number;
  }

  i64() {
    this.check(8);
    const value = this.view.getBigInt64(this.offset, true);
    this.offset += 8;
    const number = Number(value);
    if (!Number.isSafeInteger(number)) {
      throw new Error("Server value exceeds JavaScript's safe integer range.");
    }
    return number;
  }

  string() {
    const length = this.u64();
    this.check(length);
    const value = textDecoder.decode(this.bytes.subarray(this.offset, this.offset + length));
    this.offset += length;
    return value;
  }

  raw(length: number) {
    this.check(length);
    const value = this.bytes.slice(this.offset, this.offset + length);
    this.offset += length;
    return value;
  }
}

export function encodeHello(token: string, resumeFrom: number | null) {
  const writer = new Writer();
  writer.u32(0);
  writer.string(token);
  writer.u8(resumeFrom === null ? 0 : 1);
  if (resumeFrom !== null) writer.u64(resumeFrom);
  return writer.finish();
}

export function encodeMessage(chat: number, kind: MessageKind, body: Uint8Array) {
  const writer = new Writer();
  const id = crypto.getRandomValues(new Uint8Array(16));
  writer.u32(1);
  writer.raw(id);
  writer.u64(chat);
  writer.u32(kind);
  writer.u64(body.length);
  writer.raw(body);
  writer.i64(Math.floor(Date.now() / 1000));
  return { bytes: writer.finish(), id: Array.from(id, (byte) => byte.toString(16).padStart(2, "0")).join("") };
}

export function encodeTextMessage(chat: number, message: string) {
  return encodeMessage(chat, 0, textEncoder.encode(message));
}

export function encodeTyping(chat: number, on: boolean) {
  const writer = new Writer();
  writer.u32(3);
  writer.u64(chat);
  writer.u8(on ? 1 : 0);
  return writer.finish();
}

export function decodeServerFrame(bytes: Uint8Array): ServerFrame {
  const reader = new Reader(bytes);
  switch (reader.u32()) {
    case 0:
      reader.u64(); // authenticated user id
      reader.string(); // handle
      return { type: "hello", nextSeq: reader.u64() };
    case 1: {
      const envelope: Envelope = {
        seq: reader.u64(),
        chat: reader.u64(),
        sender: reader.u64(),
        sender_handle: reader.string(),
        kind: reader.u32() as MessageKind,
        body: reader.raw(reader.u64()),
        created_at: reader.i64(),
      };
      return { type: "envelope", envelope };
    }
    case 2:
      reader.u64(); // sender id
      return {
        type: "typing",
        chat: reader.u64(),
        handle: reader.string(),
        on: reader.u8() !== 0,
      };
    case 3: {
      const id = Array.from(reader.raw(16), (byte) => byte.toString(16).padStart(2, "0")).join("");
      return { type: "sent", id, seq: reader.u64() };
    }
    case 4:
      reader.u32(); // stable error code
      return { type: "error", message: reader.string() };
    case 5:
      return { type: "pong" };
    default:
      throw new Error("Server sent an unsupported Koko protocol frame.");
  }
}

export function decodeMessageText(body: Uint8Array) {
  return textDecoder.decode(body);
}
