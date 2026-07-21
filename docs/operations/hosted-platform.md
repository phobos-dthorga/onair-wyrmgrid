# Hosted platform implementation plan

## Status and scope

This is a proposal and delivery checklist, not an enabled service or a public
support claim. It covers an optional WyrmGrid website, the future WyrmGrid
Aerie community catalogue, and a separately gated private backup vault. It does
not authorize record-level synchronization, public uploads, executable-plugin
publication, a production deployment, or a release.

The governing architecture decision is
[ADR-0019](../architecture/decisions/0019-hosted-web-aerie-and-private-vault.md).
Local external-plugin delivery is governed independently by
[ADR-0020](../architecture/decisions/0020-externally-installable-extensions.md).
The legal and dependency questions are tracked in the
[hosted-platform licensing and compliance register](../legal/hosted-platform-licensing.md).

The target is a useful public presence that can start small on one dedicated
Linux server without making WyrmGrid dependent on that server. The installed
desktop application, local Hoard, existing plugins, local backups, and manual
installation of already verified packages must keep working during every
hosted-service outage.

## Design principles

- Keep public pages and forms presentational. Rust application services own
  catalogue, compatibility, authorization, moderation, signing, backup, and
  installation rules.
- Treat every upload, archive, manifest, publisher field, scanner result, and
  public URL as hostile input.
- Separate public catalogue data, quarantined uploads, publication keys, audit
  records, and private encrypted backups by service identity and storage root.
- Keep public downloads anonymous. Require identity only for durable actions
  such as publishing, moderating, or accessing private storage.
- Prefer immutable, content-addressed artifacts and replaceable stateless
  processes over mutable server directories.
- Make trust decisions verifiable by the desktop. TLS and a trusted CDN are not
  substitutes for signed repository metadata and target digests.
- Introduce no automatic update, install hook, undeclared dependency download,
  or uploaded-code execution path.
- Treat Aerie as an optional source of packages and trust metadata. It extends
  the Rust-owned offline local installer and never becomes its prerequisite.
- Add operational complexity only after measured demand. One well-separated
  host is the initial deployment target, not an assumption that one machine is
  sufficient forever.

## Proposed service boundaries

The names below describe responsibilities, not approved repository paths.
Exact packages, schemas, ports, images, and hostnames require an implementation
decision before code is added.

| Boundary                    | Initial responsibility                                                                                                                                | Explicit exclusions                                                                                                            |
| --------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| WyrmGrid Web                | SvelteKit project pages, documentation, release links, catalogue browsing, and account-facing forms                                                   | Publication decisions, permission calculation, signing, backup authorization, or direct database access                        |
| Aerie API                   | Rust HTTP adapters and application services for packages, publishers, compatibility, moderation, revocation, audit, and the public catalogue contract | Rendering the website, executing uploads, or holding offline signing keys                                                      |
| Validation worker           | Bounded structural validation and malware scanning of disposable quarantined copies                                                                   | Production credentials, container-runtime socket, general outbound network access, package execution, or publication authority |
| Hosted PostgreSQL           | Transactional catalogue, identity mapping, moderation, validation, and audit metadata                                                                 | Desktop SQLite data, raw OnAir payloads, API keys, backup plaintext, or artifact bytes                                         |
| Public artifact store       | Immutable content-addressed packages and repository metadata                                                                                          | Quarantine objects, mutable working directories, or private backups                                                            |
| Private vault API and store | Authorization and opaque storage for encrypted `.wyrmbackup` objects, if separately approved                                                          | Backup passwords, plaintext databases, catalogue objects, or record-level sync                                                 |
| Edge proxy                  | TLS termination, conservative request limits, security headers, routing, and coarse abuse controls                                                    | Application authorization or repository trust                                                                                  |

The initial deployment can use a declarative single-host container composition
behind Caddy. Only the edge proxy exposes public ports. PostgreSQL, workers,
queues or job tables, and storage control paths remain on private networks.
Kubernetes, Redis, GraphQL, WebSockets, and distributed object storage are not
initial requirements.

## Contract and repository layout decisions

Before implementation, record the selected repository layout and ownership of
these independently versioned contracts:

- public catalogue HTTP API and its OpenAPI description;
- package-envelope schema and the schema for each package kind;
- plugin protocol and SDK compatibility range;
- repository metadata and signing profile;
- hosted PostgreSQL migration series;
- desktop installation-state and rollback contract; and
- private-vault object and retention API, if that phase is approved.

Candidate code boundaries are a SvelteKit web application, a Rust Aerie service,
a Rust validation worker, shared stable Rust contract types, machine-readable
schemas and fixtures, and a deployment directory that contains no secrets.
HTTP, database, scanner, and object-store representations must be translated
into stable application types before business rules consume them.

Start with versioned REST/JSON endpoints. Use cache validators and immutable
target URLs for public reads. Uploads use bounded streaming rather than loading
whole archives into memory. Neither GraphQL nor a real-time transport should be
added until a demonstrated client workflow cannot be expressed clearly through
the versioned API.

## Website delivery

The first public phase should be a mostly static, accessibility-tested site with
project information, documentation, release verification guidance, security
contact details, legal notices, and links to official distribution channels.
It should not require an account, analytics, cookies, or a database.

If dynamic catalogue views are later introduced, SvelteKit renders the API's
public stable models and delegates every mutation to the Aerie service. Forms
use anti-CSRF protection, bounded bodies, server-side validation, and generic
error messages that do not disclose account or package internals. A restrictive
Content Security Policy, safe referrer policy, MIME sniffing protection, frame
restrictions, and dependency-pinned production builds are deployment gates.

Editorial content should remain reviewable in version control initially. A
general-purpose CMS is not needed for package authority and should not receive
catalogue, identity, signing, or vault credentials if one is ever added.

## Aerie package model

### Package classes

Themes, language packs, reference data, ordinary out-of-process plugins, and
native simulator providers have different execution and licensing risks. They
may share catalogue identity, publisher, version, licence, digest,
compatibility, moderation, and revocation concepts, but each class needs an
explicit schema, validator, permission vocabulary, and client policy.

Curated data-only assets should prove the publication pipeline before public
executable-plugin uploads are considered. Native providers require a separate
trust class and must never inherit approval merely because an ordinary plugin
with the same publisher was accepted.

### Publication lifecycle

An object progresses through explicit, auditable states:

1. **Received:** the bounded request and declared digest are recorded.
2. **Quarantined:** the immutable upload is inaccessible from public storage.
3. **Structurally valid or rejected:** deterministic validation produces a
   versioned report; failure never promotes an artifact.
4. **Awaiting review:** policy, rights, compatibility, permissions, scanner
   evidence, and publisher status are presented to an individual moderator.
5. **Approved:** an exact content digest is authorized for a named catalogue
   state through a controlled signing request.
6. **Published:** immutable bytes and signed repository metadata become public.
7. **Yanked or revoked:** new discovery or installation is stopped while
   preserving an auditable historical record and an explicit client response.

Upload is not publication. Publisher signing establishes control of a key, not
safety. Repository approval says only that the exact target passed the stated
process. User-facing language must preserve those distinctions.

### Archive and manifest validation

The package contract and shared fixtures must cover at least:

- a deterministic archive, canonical encoding, one root manifest, unique
  package ID and version, and complete content digests;
- declared package kind, entry point, runtime, permissions, network origins,
  dependencies, licence expression, notices, compatibility, and uncompressed
  inventory;
- maximum request, compressed, expanded, individual-file, file-count,
  path-length, path-depth, process, memory, CPU, disk, output, and time limits;
- rejection of absolute and parent paths, symbolic and hard links, device
  names, Windows alternate data streams, case-folding collisions, dangerous
  Unicode controls, duplicate archive entries, sparse-file tricks, and archive
  expansion bombs;
- no install scripts, package-manager hooks, imports, dynamic loading, or code
  execution during server or client validation; and
- no undeclared dependency fetch or runtime download.

The worker gets a disposable, size-limited work directory; a read-only root
filesystem; no production database, vault, signing, cloud, or orchestration
credentials; no runtime socket; restricted processes and syscalls; and no
outbound network by default. Scanner signature updates run as a different,
reviewed operation. Malware detection is one signal, never a safety proof.

The desktop repeats relevant structural, manifest, length, digest,
compatibility, and permission validation. Server validation cannot make a
download intrinsically trustworthy.

## Repository signing and desktop installation

Adopt a reviewed TUF-compatible repository profile before public executable
packages or automatic metadata refreshes are enabled. It must define root,
targets, snapshot, and timestamp roles; thresholds; expirations; consistent
snapshots; rollback and freeze behaviour; delegated publisher or package
namespaces if used; and recovery from lost or compromised keys.

Root and final publication authority stay offline or hardware-backed. The
public server may hold only narrowly scoped, replaceable online authority such
as short-lived timestamp signing if the selected profile requires it. A
documented key ceremony, named custodians, encrypted offline backups, rotation,
revocation, disaster recovery, and test repository are prerequisites. Do not
place signing keys in source control, container images, deployment variables
visible to unrelated services, or general server backups.

The Rust desktop application service performs installation as a transaction:

1. update and verify trusted repository metadata;
2. select a compatible target and verify its exact length and digest;
3. stage to a private temporary location and revalidate the archive;
4. display publisher, repository, licence, compatibility, and permission facts;
5. require explicit approval for new or expanded capabilities;
6. atomically activate the staged version without executing an install hook;
7. retain a bounded known-good rollback version and record provenance; and
8. fail closed on expired, revoked, inconsistent, or unavailable trust data
   while leaving an already installed working version usable where policy
   permits.

The Svelte callback initiating this operation remains a one-line delegation to
the Rust application service.

## Identity and authorization

Do not implement password, social-login, passkey, or account-recovery
cryptography in WyrmGrid. Use a maintained OpenID Connect provider and retain an
Aerie-owned stable publisher ID distinct from a mutable email address, display
name, GitHub identity, or Discord identity.

The desktop uses the system browser with Authorization Code and PKCE. The
embedded webview does not collect identity-provider credentials. Access tokens
are audience-restricted, minimally scoped, short-lived, and retained in the
operating-system credential store only when necessary. Refresh tokens, if
allowed, require rotation, revocation, replay handling, and a documented logout
and device-loss story.

Keep distinct scopes and server authorization checks for public reads,
publisher drafts, upload, publication requests, moderation, security response,
desktop private-vault access, and service operations. A web session cannot be
used as a moderator or vault credential. Moderators use named accounts,
phishing-resistant multifactor authentication, least privilege, step-up checks
for destructive actions, and append-only audit records. Shared administrator
accounts are prohibited.

Publisher key enrolment, rotation, compromise, loss, recovery, namespace
transfer, and account closure require explicit workflows. Account recovery
must not silently grant control of an old publisher signing key.

## Private backup vault

The least risky optional hosted-data feature is opaque storage of the existing
password-encrypted `.wyrmbackup` file. The client creates and encrypts it before
upload. The server receives neither the password nor plaintext and must not
claim that this is a zero-knowledge system until its protocol and implementation
have been reviewed.

Before launch, document and test:

- exactly which current local records the backup contains and which credentials
  or device-bound values it excludes;
- client-side key derivation, encryption, authentication, format version,
  password change, forgotten-password behaviour, and corruption detection;
- account-to-object authorization, object IDs resistant to enumeration,
  replay-safe resumable upload, exact length and digest, quotas, generations,
  retention, deletion, export, account closure, and restore across supported
  desktop versions;
- separate database roles, object roots, tokens, logs, backup sets, incident
  scope, and administrative access from the public catalogue;
- encrypted off-site copies, restore drills, deletion propagation, and the
  limits of deletion from retained disaster-recovery media; and
- minimal metadata retention, including the unavoidable account, object size,
  time, address, user-agent, billing or abuse, and access-log fields.

Operators must not be able to turn a catalogue upload grant into a backup read,
or a web account session into a desktop vault token. Support tooling should see
metadata by default, not backup bytes, and every exceptional access must be
individually authorized and audited.

### Record-level synchronization is a later system

Do not synchronize the live SQLite database or reuse opaque-backup semantics as
a sync protocol. A future design needs its own ADR, privacy impact assessment,
versioned schemas and fixtures, device identity and revocation, end-to-end key
and recovery model, provenance preservation, per-record conflict rules,
tombstones and deletion, rollback protection, quota behaviour, export, and
mixed-version compatibility.

Raw OnAir responses, OnAir API keys, provider or identity tokens,
operating-system credential-store contents, simulator recordings, private
plugin grants, local authorization decisions, and device trust state are denied
from sync by default.

## Data and storage model

Use PostgreSQL for hosted transactional state. Candidate entities include
accounts and external identity bindings, publishers and signing keys, package
namespaces and versions, artifact digests, manifests, validation runs,
moderation decisions, repository targets, yanks and revocations, scoped grants,
security events, and an audit trail. The final model must define ownership,
uniqueness, deletion, retention, and immutable history explicitly.

Hosted migrations are independently numbered and append-only after the first
public schema release. Each change receives forward, compatibility, backup,
restore, and failure tests. Row-level security may provide defence in depth but
does not replace application authorization or separate service roles.

Artifact bytes do not belong in ordinary transaction rows. Use separate,
explicit storage roots for inbound quarantine, disposable validation work,
published immutable targets, repository metadata, private vault objects, and
operational backups. Published targets use content-derived names, are never
overwritten, and receive safe content types and download disposition. Moving to
an S3-compatible store or CDN later must preserve digests, signed metadata, and
client behaviour.

## Host and network baseline

Before installation, record the actual server rather than relying on headline
capacity:

- CPU model and virtualization limits; installed and usable memory;
- SSD models, endurance, controller, redundancy, filesystem, free-space alarm,
  TRIM and SMART visibility, and replacement procedure;
- Linux distribution, supported lifecycle, kernel, time synchronization, and
  unattended-security-update policy;
- public IPv4 and IPv6, static-address guarantees, reverse DNS, DNS control,
  transfer allowance, provider filtering, SMTP restrictions, and recovery
  console access;
- provider backup, snapshot, hardware-replacement, abuse, DDoS, jurisdiction,
  and incident contracts; and
- measured inbound, outbound, disk, database, archive-validation, restore, and
  concurrent-download performance.

Only ports 80 and 443 should be public initially, with port 80 redirecting to
TLS. Administrative access uses a VPN or tightly controlled SSH allowlist,
individual keys, no password login, no direct root login, and a documented
break-glass path. Database, container runtime, metrics, storage, worker, and
management endpoints stay private.

Containers run as non-root with pinned image digests, dropped capabilities,
read-only roots where possible, explicit resource limits, dedicated service
identities, and no mounted runtime socket. Quarantine and temporary filesystems
must prevent execution and unsafe device or set-ID behaviour. Secrets come from
a protected runtime mechanism and never enter images, logs, repository files,
crash reports, metrics, browser bundles, or validation jobs.

Apply request, concurrency, account, object, and endpoint-specific rate limits.
Protect browser mutations against cross-site requests. Set conservative TLS,
HSTS only after hostname readiness, security headers, bounded log fields, and
redaction. Do not log tokens, cookies, authorization headers, backup bytes,
archive contents, API keys, private manifests, or raw OnAir data.

## Operations and recovery

### Observability

Monitor public health from outside the host and internal saturation separately.
Track request failures and latency, TLS and repository-metadata expiry,
PostgreSQL health and replication or backup age, disk and inode pressure,
validation queue depth and rejection reasons, worker resource exhaustion,
storage growth, download egress, authentication anomalies, moderation backlog,
vault failures, and restore evidence. Metrics and traces must use bounded labels
and exclude personal data and secrets.

### Backups and reconstruction

Maintain encrypted off-site backups outside the server and provider failure
domain. Cover PostgreSQL with a documented recovery-point objective, artifact
indexes and immutable public objects, configuration sufficient to rebuild the
host, audit evidence, and separately controlled vault objects. Offline signing
keys have their own custody and recovery process and do not enter routine server
backups.

Run scheduled restore drills onto a clean environment. A successful backup job
is not evidence of recoverability. The drill must prove database consistency,
object-to-digest matching, repository metadata validation, configuration
reconstruction, key separation, and a measured recovery time. Record failures
and corrective actions outside volatile server storage.

### Security and abuse response

Publish a security contact, coordinated-disclosure expectations, package-report
path, moderation policy, and service-status location before public uploads.
Prepare playbooks for compromised publisher, moderator, server, online signing
key, offline key, package, dependency, identity provider, DNS, certificate,
database, scanner, backup, and vault authorization. Each playbook must name who
can yank, revoke, rotate, rebuild, notify, preserve evidence, and restore.

Abuse handling includes spam, malware, prohibited content, impersonation,
trademark complaints, copyright notices, repeated infringement, traffic abuse,
account takeover, and denial of service. Rate limiting and malware scanning do
not replace a reachable human process.

## Delivery phases and gates

### Phase 0: decisions and rehearsal

- Confirm host, operator, jurisdiction, domain, costs, storage, backup, recovery,
  and service-level expectations.
- Complete the architecture, threat-model, privacy, legal-readiness, dependency,
  licence, package-contract, signing, and incident-response decisions.
- Build a non-public test environment with synthetic data and no production
  credentials.
- Define measurements, rollback, destroy-and-rebuild, and off-site restore
  drills.

Exit only when critical open decisions have owners, acceptance criteria, and
recorded human approval.

### Phase 1: static public website

- Publish reviewed informational pages and documentation with no accounts,
  uploads, analytics, advertising, or private data.
- Add TLS, DNS protection, security headers, accessibility checks, dependency
  inventory, reproducible deployment, external health monitoring, and tested
  rollback.

Exit when the host can be rebuilt and the site restored without relying on the
failed machine.

### Phase 2: curated read-only Aerie

- Define stable catalogue and package schemas with fixtures.
- Publish only maintainer-curated, preferably data-only targets through the
  reviewed signing process.
- Implement anonymous catalogue and download APIs plus desktop verification in
  a non-automatic, opt-in client path.

Exit after malicious, expired, rollback, freeze, mixed-version, revoked,
unavailable, corrupt, and permission-changing cases pass in server and desktop
tests.

### Phase 3: publisher identity and quarantined upload

- Deploy reviewed OIDC, publisher identity, namespace, signing-key, scoped
  authorization, audit, account-recovery, moderation, and abuse workflows.
- Accept bounded uploads only into quarantine and require human approval.
- Launch licence metadata, rights attestations, notices, takedown, yanking, and
  revocation processes.

Exit after independent security review, worker-escape testing, account and
authorization tests, moderation exercises, recovery rehearsal, and a limited
invitation-only pilot.

### Phase 4: executable community plugins

- Extend the already implemented local external-package contract with Aerie
  publisher, repository, and revocation metadata; do not create a second
  catalogue-only installation path.
- Finalize permission UX, SDK conformance, resource controls, compatibility,
  signed update verification, and revocation response.
- Do not enable automatic updates initially. Collect evidence from explicit
  user-approved installs and failure recovery.

Exit only after the plugin protocol, package schema, security documentation,
fixtures, installer and signing paths receive their required compatibility and
human approvals.

### Phase 5: opaque private backup vault

- Complete the separate privacy and security decision, encrypted format review,
  retention and deletion contract, quota model, vault API, authorization,
  restore compatibility, support boundary, off-site backup, and incident plan.
- Pilot with synthetic and disposable accounts before storing real user data.

Exit after cross-account isolation, replay, corruption, lost-password,
device-loss, deletion, restore, operator-access, breach, and provider-loss cases
pass.

### Phase 6: record-level sync

This phase is intentionally undefined. Begin it only after a separate ADR and
protocol programme are approved. It is not an incremental switch on the backup
vault.

## Verification matrix

Every phase retains the ordinary WyrmGrid quality gates and adds checks at the
lowest relevant layer:

| Area          | Required evidence before exposure                                                                                                                           |
| ------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Contracts     | Schema validation, compatibility fixtures, stable errors, size limits, downgrade and unknown-field decisions                                                |
| Rust services | Formatting, Clippy with warnings denied, unit and integration tests, dependency and licence audit                                                           |
| Web           | Svelte checking, production build, formatting, accessibility, CSP and browser security tests                                                                |
| Database      | Clean migration, upgrade, rollback strategy, failed-migration recovery, backup and point-in-time restore rehearsal                                          |
| Uploads       | Malicious archive corpus, property or fuzz testing for parsers, isolation and resource-exhaustion tests, scanner-failure behaviour                          |
| Identity      | PKCE, CSRF, audience, scope, expiry, rotation, replay, logout, recovery, privilege change, cross-account and admin tests                                    |
| Repository    | Root rotation, threshold, expiry, freeze, rollback, mix-and-match, mirror compromise, target corruption, yanking and revocation fixtures                    |
| Desktop       | Offline startup, unavailable catalogue, verified manual install, permission change, atomic failure, rollback and revoked-target behaviour                   |
| Vault         | Authenticated encryption, wrong password, corruption, quota, generation, resume, cross-account, deletion, export, versioned restore and lost-provider tests |
| Operations    | Clean-host reconstruction, off-site restore, key-loss exercise, incident tabletop, monitoring alarm and safe rollback                                       |

Hosted release and deployment automation is a critical boundary. It must be
deterministic, least-privileged, separately approved, and unable to turn
unreviewed source, optional-AI output, or an upload directly into a production
artifact. Existing rules reserving hosted CI for authorized releases or
exceptions remain in force.

## Cost planning

The preferred components can avoid recurring software-licence fees, but the
operator must budget for the service itself. Track at least domain registration,
server and transfer, off-site backups, replacement storage, transactional mail,
DNS or CDN and DDoS protection, monitoring, hardware security keys or signing
certificates, professional legal and security review, accessibility work,
moderation, incident response, and the operator's time.

Record each expense as required, optional, usage-triggered, or avoidable through
self-hosting. Self-hosting moves cost into administration and risk; it does not
make the capability free.

## Open decisions

- Who is the legal operator, in which jurisdiction, and under what project or
  organization name?
- Which host operating system, container runtime, DNS provider, domain, backup
  target, identity provider, mail provider, monitoring path, and abuse mailbox
  are acceptable?
- Which package kinds launch first, which licences are accepted, and who can
  moderate or sign them?
- What availability, recovery-point, recovery-time, retention, deletion, quota,
  transfer, and support promises can actually be sustained?
- Which signing library and key holders satisfy the approved TUF profile, and
  how are ceremonies witnessed and recovered?
- Does the first private-data feature justify its privacy, security, support,
  backup, and legal burden, or should encrypted backups remain user-managed?

No answer should be inferred solely from the current server specification.

## Reference standards

- [The Update Framework specification](https://theupdateframework.github.io/specification/)
- [OAuth 2.0 for Native Apps](https://www.rfc-editor.org/rfc/rfc8252.html)
- [OAuth 2.0 Security Best Current Practice](https://www.rfc-editor.org/rfc/rfc9700.html)
- [OWASP File Upload Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/File_Upload_Cheat_Sheet.html)
- [Caddy automatic HTTPS](https://caddyserver.com/docs/automatic-https)
- [Docker Compose production guidance](https://docs.docker.com/compose/how-tos/production/)
- [PostgreSQL row security policies](https://www.postgresql.org/docs/current/ddl-rowsecurity.html)
