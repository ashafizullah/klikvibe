# KlikVibe Remote protocol, version 1

How the KlikVibe Remote phone app talks to KlikVibe on a computer. The desktop side
is in this folder and is open source; anyone may write their own client.

## Overview

```
phone ──WebSocket (LAN)────────────────────────────▶ desktop :47615 /remote
phone ──wss (relay, Pro)──▶ klikvibe.ashafizullah.com ◀──wss── desktop (outbound)
```

Both paths carry the same thing: one Noise session per phone connection, with JSON
messages inside. The relay only forwards ciphertext; it can't read or change anything.

## Pairing

The desktop shows a QR code with this URL:

```
https://klikvibe.ashafizullah.com/link#<payload>
```

The payload is base64url (no padding) JSON:

| Key | |
|---|---|
| `v` | 1 |
| `c` | Server pairing code (from `POST /v1/pair/start`) |
| `k` | The pre-shared key for this phone: 32 random bytes, base64url |
| `n` | Computer name |
| `a` | LAN addresses (private IPv4, and 100.64/10 for Tailscale) |
| `p` | LAN port |

1. The signed-in phone claims the code at the server (`POST /v1/pair/claim`).
2. The desktop polls `GET /v1/pair/status`. Once claimed, it receives the phone's id and name, the phone's account, and its own `deviceToken` if it had none yet.
3. The desktop asks the user to allow the phone ("Allow iPhone, signed in as …?").
   - **Yes:** the desktop stores `{ phoneId, name, psk }`.
   - **No:** a computer that was just added to that account is removed again (`DELETE /v1/desktop`), and the key is never kept.

From then on the phone can connect. The key never reaches the server.

The desktop learns which phones are still allowed from the heartbeat (`POST /v1/desktop/heartbeat`, every minute while Remote is on, returns `phones: [id]`). It forgets the keys of phones that are no longer listed. A desktop on the relay also gets `{"t":"revoke","phone":id}` the moment a phone is removed.

## Connection

1. The phone opens a WebSocket. On the LAN this is `ws://<address>:<port>/remote` (no `Origin` header: requests that have one, from web pages, are refused; at most 32 connections at once). On the relay it is `wss://…/v1/relay/phone?desktop=<id>` with `Authorization: Bearer <access token>`.
2. The phone's **first binary frame** is its phone id (16 bytes, the UUID's raw bytes) followed by Noise handshake message 1.
3. Noise pattern: `NNpsk0_25519_ChaChaPoly_SHA256`, with the phone as initiator and an empty prologue.
4. The desktop finds the PSK by phone id. If there is none, or the handshake fails, it closes the connection without answering. The relay also checks that the id is the phone the access token belongs to.
5. The desktop answers with handshake message 2 (one binary frame).
6. After that, every binary frame is one Noise transport message (at most 65535 bytes). Its plaintext is one UTF-8 JSON message.

The first frame and the first transport message (`hello`) must each arrive within 10 seconds. The phone counts as connected only after its first transport message.

Either side may close at any time. The phone reconnects with a fresh handshake.

## Messages

Every message has `t`, its type. Requests that expect an answer carry `req`, a number the answer repeats.

### Phone → desktop

| `t` | Fields | |
|---|---|---|
| `hello` | `v`, `name` | Must come first. Answer: `hello` |
| `projects` | | Answer: `projects` |
| `term.open` | `id` | Start streaming a terminal: `term.reset`, the scrollback as `term.data`, then live output |
| `term.close` | `id` | Stop streaming it |
| `term.input` | `id`, `data` (string) | Typed into the terminal, as if on the keyboard |
| `git.status` | `req`, `root` | Answer: `git.status` |
| `git.diff` | `req`, `root`, `path` | Answer: `git.diff` |
| `ping` | | Answer: `pong` |

### Desktop → phone

| `t` | Fields | |
|---|---|---|
| `hello` | `v`, `name`, `os`, `app` (KlikVibe version) | |
| `projects` | `projects: [Project]` | Also sent unasked whenever something changes |
| `term.reset` | `id`, `cols`, `rows` | Clear the screen; the desktop's size (the phone never resizes the terminal) |
| `term.data` | `id`, `data` (base64) | Raw output bytes, at most 32 KiB per message |
| `term.exit` | `id`, `code` | Also when the tab is closed on the computer (`code`: null) |
| `attention` | `root`, `term`, `title`, `body` | An agent needs the user |
| `git.status` | `req`, `status`, `truncated` | `status`: `{ repo, branch, upstream, ahead, behind, changes: [{ path, from, kind, staged, unstaged }] }`. `kind`: added, modified, deleted, renamed, untracked, conflict. At most 300 changes (`truncated: true` when cut) |
| `git.diff` | `req`, `diff`, `truncated` | Unified diff, cut to fit one message (`truncated`) |
| `pong` | | |
| `error` | `req?`, `id?`, `message` | The answer to a failed request (`req`), or about one terminal (`id`: it can't be opened or typed into). Also the answer to a request whose reply would not fit one message |

`Project`:

```json
{
  "root": "/Users/me/app",
  "name": "app",
  "attention": false,
  "terminals": [{ "id": "…", "name": "Claude", "agent": "claude", "status": "busy" }]
}
```

Terminal `status` is one of:

- `idle`: a shell prompt.
- `busy`: an agent is working.
- `waiting`: an agent is waiting for the user, or has finished.
- `exited`

## Relay framing (desktop ↔ server)

The desktop keeps one connection to `wss://…/v1/relay/desktop` (header `Authorization: Device <token>`). Phones on the relay are multiplexed over it.

- **Text frames** carry control messages from the server: `{"t":"open","ch":n}`, `{"t":"close","ch":n}`, and `{"t":"revoke","phone":id}` (forget that phone's key now). The desktop sends `{"t":"close","ch":n}` to hang up one phone.
- **Binary frames** carry a 4-byte big-endian channel number followed by exactly the frame the phone sent or should receive.

The server sends a WebSocket ping every 30 seconds. The desktop treats 90 seconds without any frame as a dead link, and reconnects with backoff.
