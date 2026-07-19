# ADR-0019: Separate hosted web, Aerie, and private vault boundaries

## Status

Proposed.

## Date

2026-07-19.

## Context

WyrmGrid is local-first and does not require hosted infrastructure for ordinary
use. A future public website can nevertheless improve documentation, release
discovery, community package distribution, and optional user-controlled backup.
The existing product vocabulary already names **WyrmGrid Aerie** as the future
community catalogue.

These hosted concerns do not have one trust model:

- the public website presents reviewed project content;
- Aerie accepts hostile public metadata and eventually executable packages;
- the desktop package client verifies and installs untrusted downloads;
- publisher and moderator accounts create durable authority;
- an optional private vault would retain encrypted user-controlled backups; and
- future multi-device synchronization would introduce record conflicts,
  recovery, deletion, and metadata-leakage questions that opaque backup storage
  does not answer.

Putting those responsibilities into one SvelteKit server, database role,
storage directory, or administrator credential would allow a compromise in the
most exposed upload surface to reach private user data or publication keys. It
would also put business and security rules into the presentation layer, against
the existing WyrmGrid architecture.

The available dedicated server is expected to be adequate for a single-host
deployment, but its final operating system, storage redundancy, transfer
allowance, DDoS protection, backup service, jurisdiction, and recovery contract
are not yet confirmed. No public service is implemented or approved by this
record.

## Proposed decision

If hosted WyrmGrid services proceed, use a small set of independently
deployable boundaries:

1. **WyrmGrid Web** uses SvelteKit for presentational pages, documentation,
   catalogue views, and account-facing forms. It delegates every privileged
   decision to a Rust service.
2. **WyrmGrid Aerie API** is a Rust application service owning catalogue,
   publisher, upload, moderation, compatibility, revocation, and public API
   rules. Raw HTTP and database representations do not become desktop domain
   contracts.
3. **Aerie validation workers** receive quarantined objects through a bounded
   queue, never receive production credentials or the container-runtime socket,
   and never execute or import uploaded plugin code.
4. **Public artifact storage** is immutable and content-addressed. Quarantine,
   published packages, signing metadata, and private backups use separate roots
   and least-privileged identities.
5. **PostgreSQL** owns hosted transactional metadata. Server migrations become
   append-only after the first public hosted schema release and are versioned
   independently from desktop SQLite migrations.
6. **Identity** uses a reviewed OpenID Connect provider rather than custom
   password or passkey code. Public browsing and downloads remain anonymous.
   Publisher, moderator, desktop, and private-vault scopes remain distinct.
7. **Repository trust** separates publisher signatures from WyrmGrid repository
   approval. A TUF-compatible design provides versioned, expiring root,
   targets, snapshot, and timestamp metadata. Root and final publication
   authority remain offline or hardware-backed and unavailable to the public
   server.
8. **Desktop installation** remains a Rust application service. It verifies
   repository metadata, target length and digest, publisher identity, manifest,
   compatibility, and permissions before an atomic staged install. Svelte only
   presents the decision and result.
9. **Private backup storage**, if approved, begins by storing the existing
   encrypted `.wyrmbackup` as an opaque object. The server never receives the
   backup password or plaintext database. It is a separate authorization,
   database-role, storage, retention, and incident boundary from Aerie.
10. **Record-level synchronization** is not implied. It requires a later ADR,
    privacy impact assessment, versioned protocol and schemas, device and key
    recovery model, provenance policy, per-record conflict rules, deletion
    contract, and compatibility fixtures.
11. **Deployment** initially targets one supported Linux host with a declarative
    single-host container composition and Caddy at the public edge. Kubernetes,
    a distributed object store, Redis, GraphQL, WebSockets, and an editable CMS
    require demonstrated needs before adoption.
12. **Availability remains optional**. A website, catalogue, identity, vault,
    DNS, certificate-authority, or CDN outage must not prevent local WyrmGrid
    startup, existing plugin use, offline Hoard access, backup creation, or
    manual installation of an already verified local package.

Public package kinds may share catalogue identity, version, publisher, licence,
digest, compatibility, and moderation concepts, but each kind has an explicit
schema and validator. Data-only themes and language packs may reach curated
distribution before executable plugins. Ordinary plugins and native simulator
providers remain different trust classes.

## Trust and data separation

The intended dependency and trust direction is:

```text
browser --------> SvelteKit presentation --------> Aerie API
desktop Rust ------------------------------------> Aerie API
                                                       |
                                  +--------------------+-------------------+
                                  |                    |                   |
                              PostgreSQL          quarantine          audit trail
                                                       |
                                                isolated validator
                                                       |
                                              explicit approval/signing
                                                       |
                                              immutable public targets

desktop Rust --------> private vault API --------> opaque private objects
       |                     |
       +-- keeps key --------+-- never receives plaintext or backup password
```

The website may proxy browser requests to avoid unnecessary cross-origin
configuration. Desktop requests use the versioned public service contract
directly. A web session, catalogue upload grant, moderator role, desktop access
token, and private-vault grant never substitute for one another.

Raw OnAir responses, OnAir API keys, provider access tokens, operating-system
credential entries, plugin grants, local authorization decisions, simulator
recordings, and private operational snapshots are not public catalogue data.
The existing portable backup contains some sensitive local records and must be
described accurately if the private vault stores it.

## Package publication boundary

An uploaded object progresses through explicit states such as received,
quarantined, structurally valid, rejected, awaiting review, approved, published,
yanked, and revoked. Upload does not equal publication. Publication does not
claim that WyrmGrid proved the code safe.

The final package contract must define at least:

- a deterministic archive and canonical path encoding;
- one root manifest with independently versioned package and plugin contracts;
- immutable package ID and semantic version ownership;
- package kind, runtime, host and protocol compatibility;
- complete declared permissions, network origins, entry point, dependencies,
  licence expression, notices, publisher key, and content digests;
- compressed and expanded size, file-count, path-depth, and per-file ceilings;
- rejection of absolute paths, traversal, links, device names, alternate data
  streams, case-colliding paths, dangerous Unicode controls, and install hooks;
- no undeclared runtime dependency download; and
- fixtures for valid, invalid, malicious, yanked, revoked, expired, rollback,
  and permission-changing cases.

The validation worker treats archive parsers, malware scanners, metadata, and
publisher text as untrusted. It runs with network disabled unless a separately
reviewed scanner update operation requires it, bounded CPU, memory, disk,
process count, output, and time, and a disposable working directory. A package
is validated again on the desktop before installation.

## Identity and signing boundary

Native desktop authorization uses an external system browser with Authorization
Code and PKCE. The embedded Tauri webview does not collect identity-provider
passwords. Tokens are audience-restricted, minimally scoped, short-lived, and
stored through the operating-system credential service only when retention is
required.

Publisher identity is an Aerie-owned stable identifier bound to one or more
verified login identities and publisher signing keys. A mutable GitHub,
Discord, or email name is not the package namespace. Publisher key rotation,
loss, compromise, recovery, and revocation require an auditable ceremony.
Moderators use individual phishing-resistant authentication; shared
administrator accounts are prohibited.

Publisher signing proves control of a registered key, not code quality.
Repository signing proves that the exact target was approved for a named
catalogue state, not that it is harmless. The desktop shows both facts without
turning either into a safety guarantee.

## Licensing and cost boundary

The target implementation uses components that can be operated without
recurring software-licence fees. This is not a promise that the service has no
cost. Domains, server operation, off-site backups, mail, CDN or DDoS services,
code-signing or hardware keys, security work, professional advice, and incident
response may cost money.

Every selected source dependency, container base, image, scanner, signature
database, font, icon, screenshot, map asset, package, and external service still
requires an exact-version licence and terms review. The proposed matrix and
publication policy live in the
[hosted-platform licensing and compliance register](../../legal/hosted-platform-licensing.md).

## Consequences

- WyrmGrid reuses its Rust and Svelte expertise without making the desktop
  depend on a Node server or hosted database.
- A single host remains operationally manageable and can later place a CDN or
  S3-compatible adapter in front of immutable artifacts without changing the
  client contract.
- The design introduces serious key-management, moderation, abuse, privacy,
  backup, restoration, and incident-response obligations before executable
  uploads or private storage may launch.
- Anonymous downloads reduce account data and support burden, while publication
  and private storage remain attributable and revocable.
- A public server compromise can still disrupt availability, delete unbacked
  state, leak public submission metadata, or serve malicious bytes. Independent
  client verification and offline publication authority limit, but do not
  eliminate, those consequences.
- One physical server is a failure and compromise domain. Off-site encrypted
  backups, tested reconstruction, external availability observation, and key
  separation remain mandatory.

## Alternatives not selected

- **WordPress or a general CMS as the platform:** appropriate for editorial
  pages but not the authority for package validation, signing, authorization,
  compatibility, or private backup rules.
- **One SvelteKit full-stack application:** would place security-sensitive
  business rules in the presentation deployment and duplicate Rust contracts.
- **A generic Git forge or OCI registry as the user-facing catalogue:** useful
  as an internal transport, but insufficient on its own for WyrmGrid package
  compatibility, permissions, moderation, revocation, and desktop trust.
- **Immediate distributed object storage or Kubernetes:** adds operators and
  failure modes before a single-host workload demonstrates the need.
- **Live SQLite replication:** couples device-bound encrypted storage and local
  migrations to a hosted conflict model and risks synchronizing credentials or
  authorization state accidentally.
- **Instant unmoderated publication:** turns account compromise or upload abuse
  directly into executable-code distribution.

## Non-decisions and gates

This proposal does not select or authorize:

- a hostname, legal publisher entity, jurisdiction, privacy region, retention
  period, quota, service operator, processor, subprocessor, or launch date;
- a server operating system or exact container runtime;
- an identity provider deployment, social-login provider, SMTP service, CDN,
  object store, monitoring provider, or backup provider;
- a package format, schema number, TUF library, cryptographic algorithm, key
  holder, signing threshold, or key ceremony;
- automatic plugin updates, paid packages, donations tied to benefits, or
  private data synchronization;
- a desktop semantic-version change, plugin-protocol change, SQLite or hosted
  database migration, installer change, workflow, CI/CD deployment, release,
  tag, or public support claim; or
- acceptance of any third-party terms or professional legal advice.

The detailed delivery gates are in the
[hosted-platform implementation plan](../../operations/hosted-platform.md), the
[legal readiness checklist](../../legal/readiness.md), and the
[threat model](../../security/threat-model.md).

## References

- [SvelteKit Node deployment](https://svelte.dev/docs/kit/adapter-node)
- [Axum](https://docs.rs/axum/latest/axum/)
- [PostgreSQL licence](https://www.postgresql.org/about/licence/)
- [Caddy automatic HTTPS](https://caddyserver.com/docs/automatic-https)
- [The Update Framework specification](https://theupdateframework.github.io/specification/)
- [OAuth 2.0 for Native Apps](https://www.rfc-editor.org/rfc/rfc8252.html)
- [OAuth 2.0 Security Best Current Practice](https://www.rfc-editor.org/rfc/rfc9700.html)
- [OWASP File Upload Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/File_Upload_Cheat_Sheet.html)
