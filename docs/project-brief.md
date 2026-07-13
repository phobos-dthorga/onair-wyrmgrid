# Project brief

OnAir WyrmGrid is a modular, open-source operations and intelligence platform
for OnAir Airline Manager. Its central workspace is a universal map linking
fleet management, job intelligence, FBO network planning, maintenance, finance,
flight history, route planning, simulator integration, and community-created
extensions.

The project is local-first. It stores timestamped observations in SQLite,
supports useful offline analysis, keeps credentials on the player's machine,
and avoids requiring hosted infrastructure for ordinary use.

The core application uses Rust, Tauri, TypeScript, Svelte, MapLibre, and SQLite.
Public plugins run as separate processes over a language-neutral, versioned,
permission-controlled protocol. Python is the first intended community SDK;
Rust, C++, and TypeScript SDKs can follow once the protocol is proven.

The project grows through narrow vertical slices. The first slice is company
and fleet retrieval, validation, persistence, map display, selection, and a
non-blocking refresh path. The second proves the external plugin system with a
small idle-aircraft map layer.
