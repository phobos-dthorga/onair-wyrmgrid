# Hosted platform licensing and compliance register

## Status

This is a planning register for the proposed WyrmGrid website, WyrmGrid Aerie,
and optional private backup vault. It is not legal advice, a final bill of
materials, acceptance of third-party terms, or evidence that any hosted service
is live. It was last reviewed on 2026-07-19 and must be rechecked against the
exact versions, images, build outputs, deployment region, operator, and service
terms selected for launch.

The implementation boundaries and launch gates are documented in
[ADR-0019](../architecture/decisions/0019-hosted-web-aerie-and-private-vault.md)
and the [hosted-platform implementation plan](../operations/hosted-platform.md).

## Executive answer

The proposed core can be built and self-hosted without paying recurring
software-licence fees. That statement has important limits:

- open-source software still imposes copyright, notice, source, attribution,
  trademark, patent, and redistribution conditions;
- dependencies, container layers, scanner components, fonts, icons, and other
  assets do not automatically inherit the top-level project's licence;
- public distribution of WyrmGrid or community packages creates different
  obligations from merely running software on a private server;
- domains, hardware, bandwidth, off-site backup, email, DNS or CDN, DDoS
  protection, hardware keys, code signing, professional review, moderation, and
  incident response can cost money; and
- external services have terms, privacy roles, limits, and possible future
  charges even when their entry tier or certificate issuance is free.

Accordingly, the objective is **zero mandatory recurring software-licence
fees**, not “zero cost” or “no licensing work.”

## Candidate software register

No row approves a dependency. Record the exact version, source, licence files,
transitive dependencies, container digest, modifications, distribution method,
and required notices before selection. Prefer upstream source and reproducible
images over an unverified third-party container.

| Candidate                                           | Upstream licence or terms                                                                           | Expected fee                                  | Main obligations and cautions                                                                                                                                                                                                                                                                                                                                                                |
| --------------------------------------------------- | --------------------------------------------------------------------------------------------------- | --------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| SvelteKit                                           | [MIT](https://github.com/sveltejs/kit/blob/main/LICENSE)                                            | None for the framework                        | Retain copyright and licence text in distributed material; audit the adapter, Node runtime, JavaScript packages, generated assets, and fonts separately.                                                                                                                                                                                                                                     |
| Node.js runtime                                     | [Project licence and bundled third-party notices](https://github.com/nodejs/node/blob/main/LICENSE) | None for the upstream runtime                 | The Node licence file contains third-party terms; a container or installer may add operating-system packages with their own notices.                                                                                                                                                                                                                                                         |
| Axum                                                | [MIT](https://github.com/tokio-rs/axum/blob/main/LICENSE)                                           | None for the framework                        | Audit the complete locked Rust graph, including optional features, TLS, database, serialization, and cryptography crates.                                                                                                                                                                                                                                                                    |
| Rust toolchain and libraries                        | Mixed, predominantly permissive but dependency-specific                                             | None for upstream public releases             | Do not infer a crate's licence from the ecosystem. Preserve dual-licence choices and required notice text in the deployed and distributed bills of materials.                                                                                                                                                                                                                                |
| PostgreSQL                                          | [PostgreSQL Licence](https://www.postgresql.org/about/licence/)                                     | None for the community server                 | Permissive terms still require notice preservation. Audit drivers, extensions, backup tools, and container layers separately. Do not use a proprietary extension without a separate decision.                                                                                                                                                                                                |
| Caddy standard distribution                         | [Apache License 2.0](https://github.com/caddyserver/caddy/blob/master/LICENSE)                      | None for the upstream binary                  | Preserve licence and NOTICE material and identify modifications. Caddy modules are separately licensed; a custom build must inventory every module. Automatic HTTPS also creates certificate-authority terms.                                                                                                                                                                                |
| Moby/Docker Engine                                  | [Apache License 2.0](https://github.com/moby/moby/blob/master/LICENSE)                              | None for the open-source engine               | Audit packaged additions and registry terms. Do not assume the terms for Docker Desktop, Docker Hub, or commercial support are the same as Moby.                                                                                                                                                                                                                                             |
| Docker Compose                                      | [Apache License 2.0](https://github.com/docker/compose/blob/main/LICENSE)                           | None for the upstream plugin                  | Preserve applicable notices. Pin and inventory the delivered binary and all deployed images.                                                                                                                                                                                                                                                                                                 |
| Docker Desktop, if used on development machines     | [Docker Desktop subscription terms](https://docs.docker.com/subscription/desktop-license/)          | Depends on the user and organization          | It is not required on the proposed Linux server. Eligibility for free use and any paid subscription obligation must be assessed separately; the Engine's open-source licence does not waive Desktop terms.                                                                                                                                                                                   |
| Linux distribution and base images                  | Package-specific                                                                                    | Usually none for community distributions      | Thousands of packages may carry permissive, LGPL, GPL, and other terms. Retain package manifests and notices, minimize images, and meet source-availability obligations when redistributing images.                                                                                                                                                                                          |
| Keycloak, if selected as the OIDC provider          | [Apache License 2.0](https://github.com/keycloak/keycloak/blob/main/LICENSE.txt)                    | None for upstream Keycloak                    | Audit the exact image, Java runtime, extensions, themes, identity connectors, and base operating system. Operating it still creates security, privacy, email, recovery, and administration work.                                                                                                                                                                                             |
| ClamAV, if selected as one scanner                  | [Project COPYING file](https://github.com/Cisco-Talos/clamav/blob/main/COPYING.txt)                 | None for upstream releases                    | Much of the project is GPL-licensed and the distribution lists component-specific terms. If a binary or image is redistributed, preserve notices and satisfy applicable source obligations. Signature database access, mirrors, update tooling, and exact release terms require separate checks. Keep the scanner out of the WyrmGrid process and never treat a clean result as a guarantee. |
| TUF implementation, to be selected                  | Implementation-specific                                                                             | Normally none for open-source implementations | The [TUF specification](https://theupdateframework.github.io/specification/) defines the security model but is not the implementation bill of materials. Review the chosen library, cryptographic backend, canonical serialization, and all dependencies.                                                                                                                                    |
| Local filesystem artifact storage                   | Operating-system filesystem terms                                                                   | No separate software fee                      | It is initially simpler than an object-store product. Filesystem, encryption, backup, snapshot, and monitoring utilities remain separately licensed.                                                                                                                                                                                                                                         |
| S3-compatible storage or CDN, if later selected     | Product and provider-specific                                                                       | Often usage-based                             | An open protocol does not make a hosted provider free. Record egress, requests, retention, deletion, jurisdiction, DPA, abuse, availability, and exit terms.                                                                                                                                                                                                                                 |
| Let's Encrypt, if selected as certificate authority | [Subscriber Agreement and repository policies](https://letsencrypt.org/repository/)                 | No certificate issuance fee                   | The subscriber still accepts current terms, must protect account keys, and must comply with validation and [rate limits](https://letsencrypt.org/docs/rate-limits/). DNS, domain, and outage costs remain.                                                                                                                                                                                   |

Other candidates must be added before use. “Installed from an official package
repository,” “available on GitHub,” “free tier,” “freeware,” “source available,”
and “used only in a container” are not licence conclusions.

## Licence categories and default policy

### Permissive licences

MIT, BSD, ISC, Apache-2.0, PostgreSQL, and similar licences generally permit
commercial and modified use without a licence fee. They still require the
specified copyright, licence, and sometimes NOTICE text. Apache-2.0 also carries
express patent provisions and conditions for modified files and NOTICE content.
The exact licence text controls; a shorthand family name does not.

### Weak copyleft

LGPL, MPL, EPL, and similarly scoped licences can be acceptable, but their
linking, file-level modification, replacement, source, notice, and distribution
conditions differ. The build and packaging approach matters. Add one only after
the exact use is reviewed and compliance artifacts are part of the build.

### Strong and network copyleft

GPL software is not automatically incompatible with a hosted system. Merely
communicating with a separate GPL program does not by itself establish that all
other software shares its licence, but modification, linking, combined
distribution, container redistribution, or derivative-work questions can
create source and notice obligations. Keep GPL tools such as a scanner as
separate processes and review the exact packaging and distribution facts rather
than relying on that separation as a legal guarantee.

AGPL adds network-use source obligations and therefore requires explicit legal
and architecture approval before a hosted dependency is adopted. If compliance
would require publishing server modifications or corresponding source, the
project must intentionally accept that result and automate it.

### Source-available, non-commercial, and proprietary terms

SSPL, Business Source Licence variants, Commons Clause, non-commercial terms,
custom “ethical” restrictions, and products that only expose source are not
treated as ordinary open-source dependencies. Paid, field-of-use, user-count,
revenue, hosted-service, branding, or redistribution restrictions require a
specific approval and cost decision. Do not label a component open source
unless its licence meets an accepted open-source definition such as an
[OSI-approved licence](https://opensource.org/licenses).

## Hosted operation versus distribution

For every component, record all applicable activities:

- running an unchanged program only on the WyrmGrid server;
- modifying the program for hosted use;
- linking a library into a WyrmGrid server or desktop binary;
- copying a binary, JavaScript bundle, font, image, container, or virtual-machine
  image to users or mirrors;
- providing source, patches, scripts, written offers, notices, or installation
  information; and
- using names, logos, badges, or screenshots governed by trademark or brand
  rules.

Server-side operation may avoid some distribution-triggered obligations, while
AGPL and external service terms can attach to network use. Browser JavaScript,
downloadable packages, container publication, desktop binaries, source
archives, and public mirrors are distributions and must not be classified as
private server use.

The notices for a hosted deployment and the notices shipped with the WyrmGrid
desktop application are related but not identical. The existing
[desktop open-source licence record](open-source-licences.md) remains the
canonical record for shipped application material; candidate hosted components
must not be added there as though they are already distributed.

## Community package publication policy

Aerie becomes a distributor of user-submitted code and assets. Before accepting
an upload, require the publisher to provide and attest to:

- an SPDX licence expression for the package and every bundled component;
- complete `LICENSE`, `NOTICE`, attribution, copyright, and source-offer files;
- authority to distribute and grant the required catalogue rights for source,
  binaries, data, documentation, screenshots, icons, fonts, audio, trademarks,
  and other assets;
- a dependency and bundled-file inventory, preferably an SPDX or CycloneDX
  software bill of materials;
- the canonical source location and corresponding source for any licence that
  requires it;
- permission for WyrmGrid to host, copy, scan, transform only as technically
  required, display metadata, make backups, distribute, cache, mirror, yank,
  and retain limited audit evidence; and
- confirmation that package names and presentation do not impersonate WyrmGrid,
  OnAir, a simulator vendor, another publisher, or a third party.

No licence normally means no permission to redistribute. A public repository,
free download, or publisher checkbox does not cure missing rights. Reject
packages with absent, contradictory, fabricated, unreviewable, or incompatible
licensing evidence.

The initial catalogue should use a documented allowlist of well-understood
licences and package kinds. Proprietary, paid, source-available,
non-commercial, advertising-supported, trademark-sensitive, or unusually
restrictive packages require a later commercial and legal policy. Dependency
licences must be mutually compatible with the package's distribution and any
binary combination.

Publication terms must address publisher representations, limited licence to
operate the catalogue, ownership, warranties and disclaimers, security response,
privacy, acceptable use, export controls if applicable, sanctions if applicable,
content retention, package transfer, yanking, revocation, account closure, and
termination. A moderator's acceptance is not a warranty, endorsement, or safety
certification.

## Takedown and disputes

Before public uploads, publish a reachable process for copyright notices and
counter-notices appropriate to the operator's jurisdiction, trademark and
impersonation complaints, privacy requests, malware reports, prohibited content,
and repeated infringement. Record the legal operator, service address,
designated agent if the jurisdiction requires one, response time, evidence
retention, appeal, restoration, and emergency removal process.

Yanking and revocation have different meanings:

- **yanking** removes a version from ordinary discovery or new selection but
  preserves repository history and may leave verified existing installs usable;
- **revocation** communicates an active security or trust decision that clients
  must handle according to an approved policy; and
- **deletion** concerns stored content and personal data, which may conflict
  with audit, legal-hold, repository-consistency, and backup-retention duties.

Terms, UI, API, signed metadata, desktop behaviour, and support documentation
must use these meanings consistently.

## Plugin SDK, protocol, and independence

WyrmGrid community plugins remain out-of-process and communicate through a
versioned protocol rather than Rust, C++, Qt, Tauri, or operating-system ABI
coupling. This separation improves compatibility and containment and may
support the conclusion that independently written plugins are separate works,
but the project must not promise a universal licensing outcome. An SDK's own
licence, copied code, generated stubs, static or dynamic libraries, combined
distribution, branding, and the facts of each plugin still matter.

Publish explicit licences for the protocol specification, schemas, sample code,
SDKs, fixtures, and documentation. State what attribution generated code
requires. Avoid copying code into every plugin under unclear terms. If a
permissive SDK is selected, include its licence automatically in generated
packages and document the relationship to the application licence.

## External services and non-software terms

Before connecting any external service, add it to the
[third-party services record](third-party-services.md) with operator, purpose,
data sent, data received, credentials, legal role, region, retention, deletion,
subprocessors, contractual basis, limits, fees, outage behaviour, and exit plan.
Likely categories include:

- domain registrar, DNS and certificate authority;
- CDN, caching, DDoS protection, object storage and off-site backup;
- OIDC or social identity, passkeys, transactional mail and abuse mailboxes;
- uptime, logging, error, metrics and incident-notification providers;
- container, operating-system, malware-signature and dependency registries;
- code-signing certificate authorities, hardware-key vendors and platform
  signing or notarization programmes; and
- payment, donation or tax services if money is ever accepted.

A free tier is a pricing state, not a perpetual right. Terms, quotas, data use,
training clauses, branding, suspension, export, privacy, and pricing can change.
Pin critical behaviour behind replaceable adapters where practical and retain a
tested export and migration path.

## Data, content, and brand rights

Catalogue metadata must not expose or redistribute raw OnAir responses, API
keys, user operational data, provider tokens, or data whose public reuse has not
been established. Review the current OnAir API and brand terms before using its
name, logos, screenshots, descriptions, aircraft or airport data, or provider
responses on a public site. The public OnAir API remains read-only unless
current official documentation establishes a supported write operation.

Apply the same review to simulator and network providers, airport and map data,
tiles, weather, aircraft imagery, screenshots, video, documentation excerpts,
icons, fonts, translation sources, music, sound, user avatars, and third-party
marks. Factual data can still be protected by contractual, database, privacy,
or jurisdiction-specific rights. Link or independently create material when a
licence does not grant hosting and redistribution.

WyrmGrid branding needs a written policy before packages may imply “official,”
“verified,” “certified,” or project endorsement. Reserve project and system
names against impersonation while allowing accurate compatibility statements.

## Privacy and consumer obligations

Accounts, audit trails, access logs, upload attribution, moderation records,
support mail, security reports, and vault object metadata are personal data or
may become personal data. The operator must complete the
[legal readiness checklist](readiness.md) before collection and document:

- controller and processor roles, lawful basis, purposes, minimization,
  retention, deletion, export, correction, objection, complaints, and contact;
- countries, subprocessors, transfers, breach response, legal requests, and
  data-processing agreements;
- age restrictions and any consent requirements;
- account recovery, termination, service discontinuation, backup limitations,
  availability and loss disclaimers; and
- whether consumer, accessibility, digital-service, marketplace, tax, export,
  or content-moderation rules apply in the operator's and users' jurisdictions.

Client-side encryption reduces the plaintext exposed to the vault operator but
does not erase account, network, object, retention, support, or breach
obligations. Do not advertise “zero knowledge,” “anonymous,” “secure,”
“end-to-end encrypted,” or guaranteed recovery unless the exact claim is
supported by the implemented protocol and review evidence.

## Signing, certificates, and platform fees

Repository signing can use open-source cryptography without a per-signature fee,
but safe custody may justify hardware security keys, an HSM, secure storage, or
professional ceremony support. TLS certificates can be obtained without an
issuance fee through an ACME certificate authority, but domains, DNS, operations,
and agreement compliance still cost time or money.

Windows code signing, reputation services, Apple Developer membership and
notarization, commercial certificates, hardware tokens, timestamping, and store
distribution can impose current or future fees. They are separate from TUF
repository metadata and publisher signatures. Do not promise “signed plugins”
or a “signed installer” without naming which signature, key owner, verification
path, expiry or revocation behaviour, and trust statement is meant.

## Compliance evidence and automation

For every production deployment and distributable artifact, retain outside
volatile runtime storage:

- exact source revisions, dependency locks, selected features, build tools,
  container image digests, operating-system package manifests, and deployment
  configuration hashes;
- an SBOM for Rust, JavaScript, containers, operating-system packages, scanners,
  database extensions, static assets, fonts, and bundled community content;
- machine-readable licence conclusions plus the reviewed licence, notice,
  attribution, modification, written-offer, and corresponding-source bundle;
- vulnerability and provenance scan results with documented exceptions, owners,
  expiry, and non-security licence findings kept distinct;
- upstream source or a durable source mirror when redistribution obligations
  require availability; and
- the public notices generated from the reviewed inventory, checked against the
  actual shipped or deployed artifact rather than a developer checkout.

Use locked Rust and JavaScript dependency audits, container and OS inventory,
SBOM generation, policy checks for denied licence expressions, secret scanning,
and deterministic notice generation. Automation flags evidence for a person; it
does not determine derivative-work status, compatibility, rights ownership, or
legal sufficiency.

The scanner itself, its signature database, the identity-provider image, base
images, deployment tools, and optional monitoring agents belong in the audit.
Review every upgrade that changes licences, features, dependencies, image
layers, service terms, data flows, or notice output.

## Cost and obligation ledger

| Item                                              | Software-licence fee expectation               | Other likely cost or obligation                                                                          |
| ------------------------------------------------- | ---------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| Core web, Rust API, PostgreSQL and Caddy          | Avoidable with proposed open-source components | Administration, patching, monitoring, notices, source compliance and security review                     |
| Dedicated server                                  | Already available, subject to its contract     | Power or rental, transfer limits, storage wear, replacement, provider policy and operator time           |
| Domain and DNS                                    | Not a software licence                         | Registration and renewal; managed DNS or DDoS options may charge                                         |
| TLS certificates                                  | Can be zero-fee                                | Agreement, validation, renewal, rate limits, key protection and outage response                          |
| Off-site backup                                   | Self-hostable but not optional operationally   | Independent storage, transfer, encryption, retention, restore exercises and deletion controls            |
| Identity and mail                                 | Self-hostable open-source options exist        | Deliverability, spam and abuse handling, reputation, recovery, security, privacy and possible usage fees |
| CDN or DDoS protection                            | Optional at first, depending on provider risk  | Traffic and request charges, service terms, privacy, cache purge and exit plan                           |
| Repository and publisher signing                  | Open-source tooling can be zero-fee            | Hardware keys or HSM, ceremonies, custody, rotation, recovery and possible platform certificates         |
| Malware and supply-chain scanning                 | Open-source tools exist                        | Update bandwidth, false-positive review, isolation, source compliance and specialist testing             |
| Legal, privacy, accessibility and security review | Not a software licence                         | Professional fees or substantial qualified volunteer time; jurisdiction-dependent                        |
| Moderation and incident response                  | Not a software licence                         | Continuous human availability, evidence handling, notices, recovery and user support                     |

## Approval gates

Before Phase 1 public website launch:

- approve the operator, jurisdiction, domain and privacy baseline;
- approve and inventory exact production dependencies, images, assets and
  external terms;
- generate accurate public notices and confirm accessibility and brand rights;
- document backup, patching, incident, abuse and service-discontinuation paths.

Before any account, upload, or Aerie publication:

- approve terms of service, privacy notice, publisher agreement, acceptable-use
  policy, moderation rules, security contact, takedown and repeated-infringer
  process;
- approve exact package licences, SPDX handling, dependency evidence,
  corresponding-source retention, SDK terms and trademark rules;
- approve identity, mail, logging, retention, deletion, export, subprocessors,
  repository signing, key custody and incident response; and
- run the required contract, security, compatibility, restore and moderation
  exercises.

Before private vault storage:

- complete a privacy impact assessment and professional review appropriate to
  the operator and user regions;
- approve the encrypted format, claims, metadata, authorization, quotas,
  retention, deletion, legal hold, export, account closure, off-site backup,
  provider loss, password loss, version compatibility and discontinuation plan;
- verify that exact client and server dependency licences and cryptographic
  export or platform rules are satisfied; and
- test cross-account isolation, breach response and full restore with synthetic
  data.

Any material change to licensing, service terms, package policy, public claims,
data collection, authentication, cryptography, signing, migrations, protocol or
schema compatibility, releases, workflows, or deployment authority remains a
critical project boundary and requires the corresponding review and approval.
