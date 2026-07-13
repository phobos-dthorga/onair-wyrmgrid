# ADR-0001: Rust, Tauri, and a web map

Status: accepted

WyrmGrid uses Rust for core services, Tauri for the desktop shell, Svelte and
TypeScript for the interface, and MapLibre GL JS for the Atlas. This combination
fits asynchronous API, persistence, process supervision, and mature geospatial
rendering without making Qt deployment a project-wide dependency.

C++ remains appropriate for isolated simulator or hardware sidecars when a
native SDK makes it the clearest implementation.
