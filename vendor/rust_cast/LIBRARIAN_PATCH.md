# Librarian rust_cast patch

- Upstream: `rust_cast` 0.21.0 (`https://github.com/azasypkin/rust-cast`)
- License: MIT (see `LICENSE`)
- Local reason: upstream constructors use unbounded `TcpStream::connect` and do not expose the socket.
- Local change: `connect_without_host_verification_with_timeouts` creates the socket with an OS-level
  connect deadline and applies read/write deadlines before constructing the existing TLS/channel stack.
- Review rule: compare this directory with each upstream release at least quarterly. Remove the local
  fork as soon as upstream exposes an equivalent configured-socket or timeout API.
- Qualification: run the unreachable, accepts-but-never-responds, and disconnect-during-command Cast
  cases before updating the pinned version.
