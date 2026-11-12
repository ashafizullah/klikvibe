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
3. The desktop asks the user to allow the phone ("Allow iPhone, signed in as …?"), with a **check code**. The phone shows the same code once its claim went through:

   `n = u32_be(SHA-256("klikvibe/pair-check/1" ‖ psk ‖ phoneId)[0..4]) mod 1 000 000`, shown as two groups of three digits (`"337 965"` for psk = 32 × `0x09` and phoneId `6f0b0c2e-8f43-4a0e-9b7c-0d6a1f3a2b11`).

   Someone else who saw the QR code and claimed it first gets the desktop's prompt with their own phone's id, so another code, while the user's phone shows none (its claim failed). The user allows the phone only when the codes match.
   - **Yes:** the desktop stores `{ phoneId, name, psk }`.
   - **No:** a computer that was just added to that account is removed again (`DELETE /v1/desktop`), and the key is never kept.

From then on the phone can connect. The key never reaches the server.

The desktop learns which phones are still allowed from the heartbeat (`POST /v1/desktop/heartbeat`, every minute while Remote is on, returns `phones: [id]`). It forgets the keys of phones that are no longer listed. A desktop on the relay also gets `{"t":"revoke","phone":id}` the moment a phone is removed.

## Connection

1. The phone opens a WebSocket. On the LAN this is `ws://<address>:<port>/remote` (no `Origin` header: requests that have one, from web pages, are refused; only from private, Tailscale or loopback addresses; at most 8 TCP connections per address and 32 WebSockets at once; the request must arrive within 10 s). On the relay it is `wss://…/v1/relay/phone?desktop=<id>` with `Authorization: Bearer <access token>`.
2. The phone's **first binary frame** is its phone id (16 bytes, the UUID's raw bytes) followed by Noise handshake message 1.
3. Noise pattern: `NNpsk0_25519_ChaChaPoly_SHA256`, with the phone as initiator and an empty prologue.
4. The desktop finds the PSK by phone id. If there is none, or the handshake fails, it closes the connection without answering. The relay also checks that the id is the phone the access token belongs to.
5. The desktop answers with handshake message 2 (one binary frame).
6. After that, every binary frame is one Noise transport message (at most 65535 bytes). Its plaintext is one UTF-8 JSON message.

The first frame and the first transport message (`hello`) must each arrive within 10 seconds. The phone counts as connected only after its first transport message.

Either side may close at any time. The phone reconnects with a fresh handshake.

## Key rotation

The key from the QR code is replaced as soon as the phone connects, so it only works once: someone who saw the QR code (a screen share, a photo) can't use it after that.

1. After its `hello`, a phone that supports it asks: `{"t":"key.new","req":n}`.
2. The desktop answers `{"t":"key.new","req":n,"k":"<base64url 32-byte key>"}`. Asking again before step 4 gives the same key.
3. The phone stores it: the key this connection used stays as its key, the new one as pending. Only then does it send `{"t":"key.ok"}`.
4. The desktop makes the new key the phone's key and keeps it. The old key stops working, and the phone's other connections still on it end; this one goes on.

The phone tries its pending key first, then its key. The desktop accepts the phone's key, and also a key it offered whose `key.ok` never arrived (it then becomes the key). A desktop that lost the offer (it restarted) refuses the pending key, and the phone falls back to the key it used last. A desktop without key rotation ignores `key.new`, and the phone gives up waiting.

## Messages

Every message has `t`, its type. Requests that expect an answer carry `req`, a number the answer repeats.

### Phone → desktop

| `t` | Fields | |
|---|---|---|
| `hello` | `v`, `name` | Must come first. Answer: `hello` |
| `projects` | | Answer: `projects` |
| `term.open` | `id` | Start streaming a terminal: `term.reset`, the scrollback as `term.data`, then live output. The scrollback is the terminal as the desktop window shows it (up to 3000 lines of history, with colors and the cursor), or the last raw output when the window doesn't answer in time |
| `term.close` | `id` | Stop streaming it |
| `term.input` | `id`, `data` (string) | Typed into the terminal, as if on the keyboard |
| `git.status` | `req`, `root` | Answer: `git.status` |
| `git.diff` | `req`, `root`, `path` | Answer: `git.diff` |
| `git.stage` / `git.unstage` | `req`, `root`, `paths` | Stages (or unstages) these changed files, paths as `git.status` lists them (any other path is refused). Answer: the same `t` with `ok: true`. Only with the `git.write` feature |
| `git.commit` | `req`, `root`, `message` | Commits what is staged, or every change when nothing is (Commit all on the computer). `message`: not empty, at most 10000 bytes. Answer: `git.commit`, `ok: true`. Only with `git.write` |
| `git.push` | `req`, `root` | Pushes the branch, publishing it to `origin` when it has no upstream yet. Answer: `git.push`, `ok: true`, or `error` with Git's message. Only with `git.write` |
| `git.hunks` | `req`, `root`, `path` | Whether each hunk of that changed file's diff is staged. Answer: `git.hunks`, `staged: [bool]`. Only with the `git.hunk` feature |
| `git.hunk` | `req`, `root`, `path`, `hunk`, `action` | One hunk of the file's diff, as `git.diff` showed it (from its `@@` line to the next): `action` `stage`, `unstage` or `discard`. A hunk no longer in the diff is refused. With `action: "restore"` and `patch` instead: puts back a discarded hunk (Undo). Answer: `git.hunk`, `ok: true`, and for `discard` the `patch` to restore it. Only with `git.hunk` |
| `term.new` | `req`, `root`, `agent` | Opens a terminal tab in that open project, running `agent` (an `id` from `hello`'s `agents`) or, with `agent: null`, a plain shell. Answer: `term.new`. Only with the `term.new` feature |
| `term.kill` | `req`, `id` | Closes that terminal's tab on the computer, ending its shell and what runs in it, as closing the tab there does. Answer: `term.kill`. Only with the `term.kill` feature |
| `file.put` | `req`, `up`, `name`, `size`, `offset`, `data`, `last` | One chunk of an image to save on the computer (see [Uploads](#uploads)). Answer: `file.put`. Only with the `file.put` feature |
| `ping` | | Answer: `pong` |

### Desktop → phone

| `t` | Fields | |
|---|---|---|
| `hello` | `v`, `name`, `os`, `app` (KlikVibe version), `features`, `agents`, `prompts` | `features`: what this desktop can do beyond the first version of the protocol, `["term.new", "term.kill", "file.put", "prompts", "git.write", "git.hunk"]` (absent from older desktops: a phone offers only what is listed). `agents`: `[{ id, label }]`, the agents the desktop can start (`{ "id": "claude", "label": "Claude Code" }`); empty until the window has reported them, just after the app starts. `prompts`: the terminal's Prompts menu as the user set it on the computer (strings, in its order), for the phone to offer as ready answers; at most 100 and 24 KiB of text in all. Only with the `prompts` feature |
| `projects` | `projects: [Project]` | Also sent unasked whenever something changes |
| `term.reset` | `id`, `cols`, `rows` | Clear the screen; the desktop's size (the phone never resizes the terminal). Also sent again, with the output so far, when the terminal is resized on the computer |
| `term.data` | `id`, `data` (base64) | Raw output bytes, at most 32 KiB per message |
| `term.exit` | `id`, `code` | Also when the tab is closed on the computer (`code`: null) |
| `attention` | `root`, `term`, `title`, `body` | An agent needs the user |
| `git.status` | `req`, `status`, `truncated` | `status`: `{ repo, branch, upstream, ahead, behind, changes: [{ path, from, kind, staged, unstaged }] }`. `kind`: added, modified, deleted, renamed, untracked, conflict. At most 300 changes (`truncated: true` when cut) |
| `git.diff` | `req`, `diff`, `truncated` | Unified diff, cut to fit one message (`truncated`) |
| `term.new` | `req`, `id` | The new terminal's id: it is in the project list already (a `projects` message follows), and can be opened with `term.open` |
| `term.kill` | `req`, `id` | The terminal is closed (a `term.exit` and a `projects` message follow) |
| `file.put` | `req`, `ok: true`, `path?` | A chunk was written. After the last one, `path`: where the image is on the computer (absolute) |
| `git.stage`, `git.unstage`, `git.commit`, `git.push` | `req`, `ok: true` | Done; ask `git.status` again for the new state. Nothing here throws changes away; only `git.hunk` with `discard` does, one hunk at a time, with its Undo |
| `prompts` | `prompts` | Sent unasked when the Prompts menu changes on the computer: the whole list again, as in `hello`. Only with the `prompts` feature |
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

### New terminals

`{ "t": "term.new", "req": 4, "root": "/Users/me/app", "agent": "claude" }`

- `root` must be one of the open projects, and `agent` `null` or one of the ids in `hello`'s `agents`. The phone never sends a command: the desktop runs the command it has for that agent.
- The desktop opens the tab (and makes it the project's active terminal), waits for its shell to start, and answers `{ "t": "term.new", "req": 4, "id": "…" }`. The agent's command is typed into the new shell, as the Scripts menu does.
- It fails with `error` when the project isn't open, the agent is unknown, another `term.new` of this connection is still waiting, or the terminal hasn't started within 10 seconds.

### Uploads

An image from the phone, saved on the computer so the phone can hand its path to an agent (typed into a terminal). It comes in chunks, as one Noise message holds at most 65535 bytes:

`{ "t": "file.put", "req": 7, "up": "k3J9x2Lq", "name": "photo.jpg", "size": 51234, "offset": 0, "data": "<base64>", "last": false }`

| Field | |
|---|---|
| `up` | Chosen by the phone, the same for every chunk of one file: 8 to 40 characters of `A-Z a-z 0-9 _ -` |
| `name` | The file's name. Its extension (any case) must be `png`, `jpg`, `jpeg`, `gif`, `webp` or `heic` |
| `size` | The whole file, in bytes: at most 20 MiB (20971520), and the same in every chunk |
| `offset` | Where this chunk starts: the bytes sent so far. Chunks go in order, one at a time |
| `data` | Base64 (standard, padded): at most 36 KiB (36864 bytes) before encoding |
| `last` | `true` on the final chunk, which must end exactly at `size` |

- Each chunk is answered `{ "t": "file.put", "req": 7, "ok": true }`; the last one `{ "t": "file.put", "req": 8, "ok": true, "path": "/private/var/folders/…/klikvibe-remote/1f3a9c0e-photo.jpg" }`.
- Any error (a chunk out of order or past `size`, a bad name, too large) is answered with `error`, and the partial file is deleted: the phone starts over with a new `up`.
- At most two uploads may be in progress per connection. The partial files of a connection are deleted when it ends.
- Files go to `klikvibe-remote` in the computer's temporary folder (private to the user), named `<8 random hex>-<name>` with the name cut to 80 characters of `A-Z a-z 0-9 . _ -`. They are deleted after a week.

## Push notifications

When an agent needs the user and the account has Pro, the desktop also asks the server for a push (`POST /v1/desktop/notify`, `Authorization: Device <token>`):

```json
{ "title": "Claude Code · app", "body": "", "term": "<terminal id>", "agent": "claude" }
```

`term` is the terminal (as in `attention`), so tapping the push can open it; `agent` (agents.ts id) is left out when the terminal runs none. The body stays empty: what the agent said never passes through the server, Apple or Google.

## Relay framing (desktop ↔ server)

The desktop keeps one connection to `wss://…/v1/relay/desktop` (header `Authorization: Device <token>`). Phones on the relay are multiplexed over it.

- **Text frames** carry control messages from the server: `{"t":"open","ch":n}`, `{"t":"close","ch":n}`, and `{"t":"revoke","phone":id}` (forget that phone's key now). The desktop sends `{"t":"close","ch":n}` to hang up one phone.
- **Binary frames** carry a 4-byte big-endian channel number followed by exactly the frame the phone sent or should receive.

The server sends a WebSocket ping every 30 seconds. The desktop treats 60 seconds without any frame as a dead link, and reconnects with backoff.
