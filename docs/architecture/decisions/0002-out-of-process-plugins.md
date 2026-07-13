# ADR-0002: Out-of-process plugins

Status: accepted

Public plugins run outside the desktop process and communicate through a
versioned language-neutral protocol. They do not use Rust or C++ ABI linkage and
do not receive raw credentials.

This permits Python, Rust, C++, TypeScript, and other implementations while
providing crash isolation, dependency isolation, restartability, permission
checks, and a stable compatibility boundary.
