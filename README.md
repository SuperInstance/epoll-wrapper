# Epoll Wrapper — Safe Rust Bindings for Linux I/O Multiplexing

**epoll** is Linux's scalable I/O event notification mechanism — a system call API that lets a single thread monitor thousands of file descriptors for readiness events (readable, writable, error, hangup). This crate provides a safe Rust wrapper around the raw `epoll_create1`, `epoll_ctl`, and `epoll_wait` syscalls.

## Why It Matters

Every high-performance network server on Linux uses epoll (or a higher-level abstraction built on it). nginx, Redis, HAProxy, Envoy, and Tokio's reactor all rely on epoll for their event loop. Without I/O multiplexing, you'd need one thread per connection — and threads don't scale to 100K connections. With epoll, a single thread can handle `C10K` and beyond. The alternative APIs — `select(2)` (1024 FD limit, `O(n)` scan) and `poll(2)` (`O(n)` scan) — don't scale. Epoll is `O(1)` for active events: `epoll_wait` returns only the descriptors that are actually ready.

## How It Works

### Epoll Lifecycle

```
1. epoll_create1(flags)    → epfd  (create epoll instance)
2. epoll_ctl(ADD, fd, ev)  → add fd to interest list
3. epoll_wait(events, ms)  → block until ≥1 fd ready, return events
4. epoll_ctl(MOD, fd, ev)  → change what events we care about
5. epoll_ctl(DEL, fd)      → remove fd from interest list
6. close(epfd)             → destroy epoll instance
```

### Event Types

| Flag | Constant | Meaning |
|---|---|---|
| `EPOLLIN` | Data available to read |
| `EPOLLOUT` | Ready for writing |
| `EPOLLERR` | Error condition |
| `EPOLLHUP` | Hang up (peer closed) |
| `EPOLLET` | Edge-triggered (see below) |

### Level-Triggered vs. Edge-Triggered

**Level-triggered (default)**: `epoll_wait` notifies as long as the FD is ready. If you don't read all available data, the next `epoll_wait` will notify again. Simpler to use.

**Edge-triggered (`EPOLLET`)**: `epoll_wait` notifies *only on the transition* — when new data arrives. If you don't drain the socket completely, you won't get another notification. This is more efficient (fewer syscalls) but requires non-blocking I/O and careful drain loops:

```
while read(fd) != EAGAIN { ... }  // must drain until EAGAIN
```

### Performance Model

- `epoll_ctl(ADD/MOD/DEL)`: `O(1)` — hash table insert/modify/delete in kernel.
- `epoll_wait`: `O(ready_events)` — only returns descriptors with events, not all registered FDs.
- Memory: `O(registered_fds)` kernel-side interest list.

This is why epoll scales: the cost is proportional to *active* events, not *registered* descriptors.

## Quick Start

```rust
use epoll_wrapper::{Epoll, EpollEvent};

let ep = Epoll::new()?;

// Add a socket for reading (edge-triggered)
ep.add(socket_fd, &[EpollEvent::In, EpollEvent::EdgeTriggered], data=42)?;

// Wait for up to 64 events, 1000ms timeout
let mut events = vec![libc::epoll_event::default(); 64];
let n = ep.wait(&mut events, 1000)?;

for i in 0..n {
    let ev = &events[i];
    println!("FD data={} ready", ev.u64);
}

// Modify or remove
ep.modify(socket_fd, &[EpollEvent::In, EpollEvent::Out], 42)?;
ep.delete(socket_fd)?;
// epfd is automatically closed on Drop
```

## API

| Method | Description |
|---|---|
| `Epoll::new()` | Create epoll instance (`epoll_create1`). `O(1)`. |
| `ep.add(fd, events, data)` | Register FD (`EPOLL_CTL_ADD`). `O(1)`. |
| `ep.modify(fd, events, data)` | Modify registration (`EPOLL_CTL_MOD`). `O(1)`. |
| `ep.delete(fd)` | Remove FD (`EPOLL_CTL_DEL`). `O(1)`. |
| `ep.wait(events, timeout_ms)` | Block for events → count. `O(ready)`. |
| `EpollEvent` | Enum: `In`, `Out`, `Err`, `Hup`, `EdgeTriggered`. |

**Safety**: The wrapper manages the raw `epfd` via RAII — `Drop` closes the FD. Individual `epoll_ctl` calls are marked `unsafe` internally and validated via return codes.

## Architecture Notes

Epoll is the I/O substrate for the γ (generation/I/O) side of γ + η = C in SuperInstance. It powers the event loops that handle fleet instance networking, enabling high-concurrency I/O without thread-per-connection overhead. See [SuperInstance Architecture](https://github.com/SuperInstance/SuperInstance/blob/main/ARCHITECTURE.md).

## References

1. Linux epoll(7) man page. <https://man7.org/linux/man-pages/man7/epoll.7.html>
2. Corbet, J., Rubini, A., & Kroah-Hartman, G. (2005). *Linux Device Drivers* (3rd ed.), Ch. 6. O'Reilly.
3. Libuv Design. <https://docs.libuv.org/en/v1.x/design.html> — How libuv uses epoll internally.

## License

MIT
