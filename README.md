[![progress-banner](https://backend.codecrafters.io/progress/http-server/ded4ad65-cb97-47d5-85e1-b64a8aa80b27)](https://app.codecrafters.io/users/PhuThanh2211?r=2qF)

# HTTP/1.1 Server from Scratch (Rust)

An HTTP/1.1 server built on raw TCP (`std::net`), with no HTTP library. It started as the
[CodeCrafters "Build Your Own HTTP server"](https://app.codecrafters.io/courses/http-server/overview)
challenge and grew into a small but complete server: routing, static files, caching,
range requests, compression, virtual hosting, graceful shutdown and a WebSocket handshake.

Each section below follows the same shape: **Problem → Solution → Advantages → Example → Trade-offs**.

## Contents

1. [Project layout](#project-layout)
2. [Running and testing](#running-and-testing)
3. [Request parsing and limits](#1-request-parsing-and-limits)
4. [Routing (static, pattern, 404 vs 405)](#2-routing)
5. [Query strings](#3-query-strings)
6. [Static files, path safety and MIME types](#4-static-files-path-safety-and-mime-types)
7. [Conditional requests and ETags](#5-conditional-requests-and-etags)
8. [Range requests](#6-range-requests)
9. [Gzip compression](#7-gzip-compression)
10. [Persistent connections and timeouts](#8-persistent-connections-and-timeouts)
11. [Concurrency and graceful shutdown](#9-concurrency-and-graceful-shutdown)
12. [Error pipeline and panic recovery](#10-error-pipeline-and-panic-recovery)
13. [Access logging](#11-access-logging)
14. [Virtual hosting](#12-virtual-hosting)
15. [WebSocket upgrade handshake](#13-websocket-upgrade-handshake)
16. [Known limitations](#known-limitations)

---

## Project layout

| Path | Responsibility |
|---|---|
| `src/main.rs` | Accept loop, per-connection thread, keep-alive loop, shutdown, panic recovery, logging wiring |
| `src/request.rs` | Request parsing (request line, headers, body, query string), size limits |
| `src/response.rs` | Response builders: `ok`, `error` and its wrappers (400 to 504), `not_modified`, `partial_content`, `switching_protocols` |
| `src/router.rs` | `Router` (exact and pattern routes), `route`, `route_vhost`, dynamic prefixes (`/echo/`, `/files/`) |
| `src/router/handlers/*.rs` | Handlers: `basic` (index, echo, user-agent, websocket), `files`, `users` |
| `src/router/tests.rs` | Routing tests (handler tests live beside their handler) |
| `src/static_files.rs` | Path normalisation and containment, MIME table, ETag helpers |
| `src/range.rs` | `Range` header resolution (206/416 decision) |
| `src/vhost.rs` | Host header matching (exact, wildcard, longest suffix) |
| `src/websocket.rs` | `Sec-WebSocket-Accept` computation and handshake validation |
| `src/logging.rs` | Common Log Format access log |
| `src/bin/*_kata.rs` | Standalone stdin exercises (etag, range, pipeline, vhost, websocket) that reuse the core modules |

## Running and testing

```sh
# Run (use `--` so cargo does not swallow the flags)
cargo run -- --directory /tmp/files

# With virtual hosts (optional, repeatable)
cargo run -- --directory /tmp/files \
  --vhost blog.example.com=/sites/blog \
  --vhost *.example.com=/sites/fallback

# Unit tests
cargo test

# Katas read stdin, for example:
echo "1000|-100" | cargo run --bin range_kata
```

On Windows PowerShell use `curl.exe` (plain `curl` is an alias of `Invoke-WebRequest`).

---

## 1. Request parsing and limits

**Problem.** A TCP socket delivers a stream of bytes with no message boundaries. A request may
arrive in several packets, the body may be binary, and a malicious client can send endless
data to exhaust memory.

**Solution.** `parse_request` reads the first chunk, splits the request line
(`METHOD PATH VERSION`), parses headers into a map with lowercase keys, then keeps reading
until `Content-Length` bytes of body have arrived. The header section is decoded as text
while the body stays raw bytes (binary-safe). Constants cap the request line (8 KB), each
header (8 KB), header count (100) and body size (10 MB).

**Advantages**
- Case-insensitive header lookup via `req.header("content-type")`.
- Binary-safe bodies (uploads, images).
- Bounded memory per request, which blunts Slowloris-style abuse.

**Example**
```
POST /files/a.txt HTTP/1.1
Content-Length: 5

hello          -> body == b"hello"
```

**Trade-offs**
- The first read uses a fixed 1 KB buffer, so very large header blocks are not handled
  completely. A `BufReader` with line-based reading would be more robust.
- No chunked transfer-encoding request bodies.
- The `headers` map keeps one value per name. `Request.host_count` exists specifically
  because duplicate `Host` headers cannot be seen in a map.

---

## 2. Routing

**Problem.** The server must turn `(method, path)` into a function call, and answer
correctly when the path exists but the method does not.

**Solution.** `Router` stores `path -> method -> handler`.
- Exact routes are looked up first, then pattern routes such as `/users/{id}/posts/{post}`
  (segments are matched in lockstep, `{name}` captures one segment, segment counts must be equal).
- Static beats dynamic regardless of registration order.
- Unknown path gives **404**. Known path with another method gives **405 + `Allow`** header.
- `HEAD` is delegated to `GET` and the body is stripped (`strip_body`).
- Registering the same `(method, path)` twice panics at startup, because it is a programmer error.
- Dynamic prefixes (`/echo/{str}`, `/files/{name}`) are tried before the table.

**Advantages**
- Adding a pattern route can never silently steal an existing literal route.
- Clients get an accurate distinction between "wrong URL" and "wrong verb".
- Handlers stay small: they receive the parsed `Request` and the document root.

**Example**
```
GET  /users/42        -> get_user  (id = "42")
GET  /users/new       -> literal route wins if registered
POST /about           -> 405, Allow: GET
GET  /missing         -> 404
```

**Trade-offs**
- Path params are always strings. Handlers must validate and convert (`/users/abc` routes
  fine, and the handler decides 400 or 404).
- A linear scan over pattern routes is fine for a handful of routes but not for thousands
  (a trie would be better).
- The routes are registered in code, not loaded from configuration.

---

## 3. Query strings

**Problem.** `/search?q=hello%20world&page=2` carries inputs after the `?`. The query must
never affect which route matches, and a naive `split('&')` mishandles encoding and empty values.

**Solution.** `request.rs` strips the query before routing and parses it into
`Vec<(String, String)>` in original order. It percent-decodes (`%20`) and treats `+` as a space.
`sort=` yields an empty-string value (present, not absent). Duplicate keys are kept and
`query_get` uses a **last-wins** policy.

**Advantages**
- Routing is independent of the query.
- Order preserved, so handlers and logs see exactly what the client sent.
- Empty and duplicate values are explicit rather than accidental.

**Example**
```
GET /search?q=cron&limit=10     -> query_get("q") == Some("cron")
GET /search?tag=a&tag=b         -> query_get("tag") == Some("b")
```

**Trade-offs**
- Bare keys (`?debug`) and multi-value access (all `tag` values) need extra code.
- Last-wins is a policy choice. Some APIs prefer first-wins or a list.

---

## 4. Static files, path safety and MIME types

**Problem.** Serving files turns attacker-controlled text into a filesystem path.
`/files/../../etc/passwd` must not escape the document root, and a browser needs the right
`Content-Type` to render or execute a file.

**Solution.**
- `normalize` resolves `.` and `..` lexically (posix style), and `resolve_safe_path` accepts the
  result only if it equals the root or starts with `root + "/"`. The trailing separator stops
  the sibling bypass (`/var/www-backup` is not inside `/var/www`).
- `resolve_on_disk` canonicalises the real root and candidate, so symlinks pointing outside
  the root are rejected too. A not-yet-existing file (POST upload) is allowed if its parent is inside.
- `mime_type_for` maps extensions (`html`, `css`, `js`, `json`, `png`, `jpg`, `gif`, `svg`, `txt`)
  to types, with `charset=utf-8` on text, and defaults to `application/octet-stream`.

**Advantages**
- Path traversal and sibling-prefix attacks return **403**.
- Unknown types are downloaded instead of rendered, which avoids an XSS hole from guessing `text/html`.
- Pure string logic is unit-testable without touching the disk.

**Example**
```
root=/var/www  /static/main.css   -> /var/www/static/main.css | text/css
root=/var/www  /../etc/passwd     -> 403
root=/var/www  /a/../index.html   -> /var/www/index.html      (interior .. stays inside)
```

**Trade-offs**
- Lexical checks cannot see symlinks, hence the extra canonicalisation step.
- On Windows `canonicalize` returns `\\?\C:\...` (verbatim) paths, so compare only
  canonicalised-to-canonicalised values and avoid pushing Windows roots through `normalize`.
- The whole file is read into memory per request, so very large files need streaming.
- User-uploaded SVG is script-capable, so serve uploads from a separate origin in production.

---

## 5. Conditional requests and ETags

**Problem.** Browsers and CDNs re-download unchanged files, wasting bandwidth and time.

**Solution.** The `GET /files/{name}` handler computes a strong **ETag**
(`"` + hex SHA-1 of the bytes + `"`, from `etag_for`) and sends it on 200. When the client replies with
`If-None-Match`, `should_return_304` decides: empty header means no, `*` means yes, otherwise
a comma-separated list is checked with `W/` weak prefixes stripped. A match returns **304 Not Modified**
with the same `ETag`, no body and no `Content-Length`.

**Advantages**
- Repeat requests become a tiny headers-only response.
- Content-based, so it stays correct even if the file's timestamp changes without a content change.
- Computed before gzip, so the same resource keeps one ETag whatever the encoding.

**Example**
```
GET /files/app.css                      -> 200, ETag: "a1b2..."
GET /files/app.css  If-None-Match: "a1b2..."   -> 304
GET /files/app.css  If-None-Match: "x", "y", "a1b2..."  -> 304
```

**Trade-offs**
- Hashing the whole file on every request costs CPU. Production servers cache the hash or
  derive it from mtime and size.
- Only `If-None-Match` is implemented. `If-Modified-Since`, `Last-Modified` and `If-Match` /
  412 for `PUT` concurrency are not.

---

## 6. Range requests

**Problem.** Large downloads and video seeking need a slice of a file, not the whole thing,
and a broken download should resume.

**Solution.** `resolve_range(size, spec)` in `range.rs` handles `bytes=a-b`, `a-` and `-N`
(suffix). Both ends are inclusive, so `length = end - start + 1`. An end past the file is clamped,
a suffix longer than the file returns the whole file, and a start beyond the end or a reversed pair
returns **416** with `Content-Range: bytes */size`. A valid slice returns **206** with
`Content-Range: bytes start-end/size` and a `Content-Length` equal to the slice length. Normal
200 responses advertise `Accept-Ranges: bytes`.

**Advantages**
- Resumable downloads and seekable media.
- A 416 tells a client with a stale size exactly what to ask for next.
- The logic is a pure function with unit tests and a stdin kata.

**Example**
```
size 1000, bytes=0-99     -> 206 0-99/1000  (100 bytes)
size 1000, bytes=-100     -> 206 900-999/1000
size 1000, bytes=2000-    -> 416 Content-Range: bytes */1000
```

**Trade-offs**
- Only a single range is served. Multi-range requests fall back to a full 200, which also
  avoids multipart amplification.
- The handler still reads the whole file before slicing. `seek` plus `take` would avoid that.
- The ETag/304 check runs before Range. `If-Range` is not implemented.

---

## 7. Gzip compression

**Problem.** Text responses are large and compress very well.

**Solution.** If `Accept-Encoding` lists `gzip`, the echo handler compresses the body with `flate2`
and sets `Content-Encoding: gzip` and the compressed `Content-Length`.

**Advantages**
- Smaller transfers with no change on the client side.

**Example**
```sh
curl.exe -H "Accept-Encoding: gzip" http://localhost:4221/echo/abc --output out.gz
```

**Trade-offs**
- CPU cost per request (cache compressed variants in production).
- Compression is only wired into the echo handler.
- Tiny bodies can grow, since gzip adds a header.
- Caches should receive `Vary: Accept-Encoding`, which is not sent yet.

---

## 8. Persistent connections and timeouts

**Problem.** Opening a TCP connection per request is slow, and an idle or silent client
can hold a thread forever.

**Solution.** Each connection thread runs a request loop (HTTP/1.1 keep-alive). The loop ends
when the client closes, a read fails, a write fails, or the request had `Connection: close`
(the server then adds `Connection: close` to the response and calls `shutdown`). A read timeout
(`set_read_timeout`) makes a silent client's read fail instead of blocking forever.

**Advantages**
- Many requests share one TCP handshake.
- Idle connections free their threads.

**Example**
```sh
curl.exe http://localhost:4221/ --next http://localhost:4221/echo/hi
```

**Trade-offs**
- Only a read timeout exists, with no write, idle or total-request deadline.
- A thread per connection does not scale to tens of thousands of idle clients (async I/O would).

---

## 9. Concurrency and graceful shutdown

**Problem.** One slow client must not block others, and killing the process on a deploy drops
in-flight requests.

**Solution.** Each accepted connection runs in `thread::spawn`. The listener is non-blocking and polls
every 100 ms, which lets the loop check a shared `AtomicBool` set by the `ctrlc` handler. On
shutdown the server stops accepting, then joins all thread handles so in-flight requests finish.

**Advantages**
- Simple model, with true parallelism between connections.
- Deploys and restarts do not produce a burst of failed requests.

**Example.** Press Ctrl+C while a request is running. The output shows "Waiting for N in-flight request(s)" and then "Shutdown complete".

**Trade-offs**
- Joining handles has no deadline, so a stuck connection can delay exit forever.
- The `handles` vector grows with every connection (finished handles are not pruned).
- The 100 ms poll adds up to that much latency to accepting a connection.

---

## 10. Error pipeline and panic recovery

**Problem.** A handler bug must not kill the connection thread or leak internals, and client
mistakes (4xx) must be distinguishable from server faults (5xx).

**Solution.** `response::error(status, reason, body, extra_headers)` backs a family of wrappers (400, 401,
403, 404, 405 with `Allow`, 408, 409, 413, 414, 415, 429, 500, 502, 503, 504). The router call is wrapped in
`catch_unwind`: a panic is logged on the server and the client gets a bare **500**.

**Advantages**
- One consistent response format.
- Error graphs stay meaningful: bad input is 4xx, our bugs are 5xx.
- No stack traces, paths or messages leak to clients.

**Example**
```sh
curl.exe -i http://localhost:4221/crash   # 500 "Internal Server Error", server keeps running
```

**Trade-offs**
- `catch_unwind` works only with unwinding panics (not `panic = "abort"`).
- A panic while holding a lock could poison it. The code currently holds none across handlers.

---

## 11. Access logging

**Problem.** Operators need to see who requested what, what was answered and how long it took.

**Solution.** `logging::log_request` writes one Common Log Format line after the handler returns:
client IP, timestamp (`chrono`), request line, real status, body byte count, user agent and latency in ms.
Status and size are extracted from the actual response bytes, so the log shows what was sent, not what was intended.

**Advantages**
- Works with standard log-analysis tools.
- Status in the log always agrees with the wire.

**Example**
```
127.0.0.1 - - [04/Oct/2026:19:30:00 +0700] "GET /files/a.txt HTTP/1.1" 200 5 "curl/8.0" 1ms
```

**Trade-offs**
- Written to stdout synchronously, so a slow terminal can slow requests.
- The line is built just before the write, so a failed write is still logged as sent.

---

## 12. Virtual hosting

**Problem.** One IP and port must serve many sites. A TCP connection carries no site name, so
without a selector you would need one IP address (or process) per site.

**Solution.** The mandatory HTTP/1.1 `Host` header selects the site. `VHosts` (`vhost.rs`) holds exact
patterns and `*.suffix` wildcards. `normalize_host` lower-cases the value and strips `:port`
(IPv6 `[::1]:8080` becomes `[::1]`). `resolve` picks an exact match first, then the wildcard with the
**longest literal suffix**. `*.example.com` never matches the bare `example.com`.
`route_vhost` returns **400** for a missing, empty or duplicate `Host` (the parser counts them in
`Request.host_count`), **404** for no match, and otherwise calls the normal router with that site's
document root. If no `--vhost` flag is given the feature is off and the server behaves as before.

**Advantages**
- Many sites share one IP, one process and one listener.
- Adding a site is a config line, with no new hardware.
- Wildcards support unlimited subdomains (SaaS tenants).
- Same path, different resource per host (`/admin` on blog and shop are different).

**Example**
```sh
cargo run -- --directory /tmp --vhost blog.example.com=/sites/blog --vhost *.example.com=/sites/fallback
curl.exe -H "Host: blog.example.com"  localhost:4221/files/index.txt   # blog files
curl.exe -H "Host: other.example.com" localhost:4221/files/index.txt   # fallback files
curl.exe -i -H "Host: nope.test" localhost:4221/                       # 404
```

**Trade-offs**
- Sites share CPU, memory and the thread pool, so a noisy neighbour affects the others.
- `Host` is client-controlled: match it only against the configured list.
- HTTPS would need SNI before `Host` is readable, and TLS is not implemented here.
- Logs are shared and not split per site yet. There is no explicit default site (unknown hosts get 404).

---

## 13. WebSocket upgrade handshake

**Problem.** HTTP is request/response, so the server cannot push data. Polling wastes bandwidth and adds latency
for chat, dashboards and games.

**Solution.** `GET /chat` with `Upgrade: websocket` asks to switch protocols. `validate_handshake`
requires: method `GET`, `Upgrade: websocket` (case-insensitive), `Connection` containing `upgrade`,
a non-empty `Sec-WebSocket-Key`, and `Sec-WebSocket-Version: 13`. On success `accept_key` computes
`base64(sha1(key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"))` (raw 20-byte digest, not hex) and the server
replies **101 Switching Protocols** with that `Sec-WebSocket-Accept`. Anything invalid gives **400**.
After a 101 the connection loop stops parsing HTTP.

**Advantages**
- Runs on the normal HTTP port and reuses cookies, auth, `Host` routing and TLS termination.
- The accept hash proves the server understands WebSocket, so a generic server or cache cannot answer by accident.
- Validation happens before committing to the new protocol.
- Frames are cheap afterwards (2 to 14 byte headers, versus hundreds of bytes of HTTP headers per message).

**Example**
```sh
curl.exe -i -N -H "Connection: Upgrade" -H "Upgrade: websocket" \
  -H "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==" -H "Sec-WebSocket-Version: 13" \
  http://localhost:4221/chat
# HTTP/1.1 101 Switching Protocols ... Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=
```

**Trade-offs**
- Only the handshake exists. Frame parsing, masking, ping/pong and close are not implemented, and the
  server closes the connection right after the 101.
- Long-lived connections fit poorly with thread-per-connection and with graceful shutdown.
- `Origin` is not checked, which a real deployment needs against cross-site hijacking.
- Server-Sent Events are simpler when only server-to-client push is needed.

---

## Known limitations

- Fixed-size first read in the parser. No chunked request bodies.
- Whole files are loaded in memory (range and ETag included).
- Thread-per-connection with no thread pool or connection cap.
- No TLS, HTTP/2 or `If-Modified-Since`.
- Timeouts are read-only (no write, idle or total-duration limits).
- WebSocket frames are not implemented, only the handshake.

## CodeCrafters

Run `codecrafters submit` to submit a solution. `./your_program.sh` builds and runs the server
(requires `cargo`). Keep the `--vhost` flags off when running the CodeCrafters tests, because they send
`Host: localhost:4221` and expect normal responses.
