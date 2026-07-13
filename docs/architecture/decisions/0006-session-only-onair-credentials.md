# ADR-0006: Begin with session-only OnAir credentials

- Status: Accepted
- Date: 2026-07-14

## Context

WyrmGrid needs authenticated read-only OnAir data before the first fleet slice
can be tested. Persisting a credential immediately would add platform-specific
credential-store integrations, permission behavior, migration concerns, and
packaging tests before the API boundary itself has been proven.

The project is currently maintained by one developer. Security matters, but an
unnecessary multi-platform account system would make the first useful test
substantially harder to maintain.

## Decision

The initial desktop connection is session-only:

- the user enters a company UUID and company-specific API key copied from
  **OnAir Client → Options → Global Settings**;
- until credential parity is verified, OnAir Companion is excluded as a source
  because its displayed values failed an authenticated public-API test on
  2026-07-14;
- Tauri forwards them to a thin Rust application command;
- Rust wraps the trimmed key in `SecretString` and validates it with the
  read-only company endpoint;
- only the authenticated client and translated company summary remain in
  process memory;
- Disconnect and process exit drop the session;
- no key is written to SQLite, browser storage, logs, fixtures, or plugins.

Errors cross the UI boundary only as predefined, non-sensitive messages. The
remote response body is not relayed to the interface.

## Consequences

Real API behavior can now be tested without committing to persistent account
management. Users must reconnect after every application restart. A future
"remember this connection" feature requires a separate decision and reviewed
operating-system credential-store implementations for each supported platform.

This is secret minimization, not a claim that process memory is a secure vault.
Crash dumps and a compromised operating system remain outside the protection
provided by this milestone.

OnAir Companion is expected to become the primary client. This source rule is
therefore transitional rather than architectural: revalidate it when Companion
reaches API credential parity, then update guidance without changing the
session-only security decision.
