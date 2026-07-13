# OnAir WyrmGrid Privacy Notice

**Version and effective date:** 2026-07-14

This preliminary notice describes information handled by official builds of
OnAir WyrmGrid distributed by Phobos A. D'thorga. It does not describe an
unofficial fork, community plugin, or a third party's own service. WyrmGrid is
still in foundation-stage development. This notice must be updated and
professionally reviewed before a stable or commercial release.

## Privacy at a glance

- WyrmGrid is local-first and has no WyrmGrid user account or advertising.
- Your OnAir API key is used for the active connection and is not written to the
  WyrmGrid database.
- Atlas downloads its current map style and tiles from MapLibre's public demo
  service after legal onboarding is complete.
- Privacy-filtered error diagnostics are optional, off by default, and only
  operate when both the user and the official build enable Sentry.
- WyrmGrid does not sell personal information.

## Information kept locally

WyrmGrid currently keeps the following information on the user's device:

- the accepted Terms and Privacy Notice versions, the optional diagnostics
  preference, and acknowledgement timestamps in WyrmGrid's SQLite database;
- interface preferences, such as the selected automatic synchronization
  interval, in the desktop webview's local storage; and
- while the application is running, the supplied OnAir company ID, API key,
  translated company details, and current fleet observation in process memory.

The API key is cleared when the OnAir session disconnects or the process exits.
Normal session-only handling cannot guarantee removal from operating-system
crash dumps, virtual memory, or a compromised computer.

## Connections to other services

### OnAir

When the user chooses to connect, WyrmGrid sends the company ID and API key
directly to OnAir's public API and requests the selected company information.
Subsequent synchronization requests retrieve fleet information. OnAir operates
independently under its own terms and privacy practices. WyrmGrid does not send
the API key to Sentry, map services, or plugins.

### MapLibre demo infrastructure

After onboarding, Atlas currently downloads a map style, fonts, and vector
tiles from `demotiles.maplibre.org`. Like any internet service, that server and
intermediate network providers receive connection metadata such as the source
IP address, request time, requested resource, and user-agent information.
WyrmGrid does not intentionally put an OnAir API key, company ID, fleet record,
or selected aircraft into those requests. This public demonstration service
must be replaced or formally approved before stable release.

### Optional Sentry diagnostics

If the user opts in and the build has been deliberately configured for public
telemetry, WyrmGrid may send unexpected Rust failures and unhandled interface
errors to separate Sentry projects in Sentry's United States data region.
Expected authentication, input, rate-limit, offline, and optional-integration
conditions are not reported.

Client-side filters remove user, request, breadcrumb, extra-data, local-path,
source-line, and unapproved context fields. A report may retain a WyrmGrid error
code, application release and environment, operating-system/runtime/application
context, safe application filenames or URLs, symbolic function names, and a
random report identifier. Sentry and its network providers necessarily receive
connection metadata during transmission. WyrmGrid must enable Sentry's
server-side IP-address scrubbing before diagnostics are enabled in a public
build.

Diagnostics can be disabled at any time in **Privacy & Terms**. Disabling stops
future reports; it does not automatically delete reports already retained by
Sentry. Current public release jobs do not embed Sentry endpoints. Before that
changes, this notice will be revised to state the configured retention period,
access controls, deletion process, and current subprocessors, and its version
will be increased.

## Purposes

WyrmGrid handles information to provide user-requested OnAir connectivity,
display operational context, remember local choices, secure and diagnose the
application, and improve reliability. Information is not used for behavioural
advertising, data brokerage, or unrelated user profiling.

## Retention and deletion

Session-only credentials and fleet state are discarded when the process exits.
Local preferences remain until changed, removed by a future reset function, or
deleted with the application's local data. Optional diagnostic events follow
the Sentry retention configuration disclosed when public telemetry is enabled.
Uninstallers and operating systems may not remove every application-data file;
users can request instructions for locating it.

## Security and limits

WyrmGrid minimises diagnostic fields, keeps credentials out of plugins, uses
encrypted HTTPS connections to external services, and treats remote data as
untrusted. No system can promise absolute security. The threat model documents
known boundaries and remaining work.

## Choices and requests

Users can decline optional diagnostics without losing the core application,
change that preference later, disconnect OnAir, and delete local application
data. Questions or requests can be raised through the project repository. Use
GitHub private vulnerability reporting for sensitive privacy or security
matters and never include a real API key. A dedicated non-public privacy contact
must be established before broad public distribution.

Depending on applicable law, a user may have rights to information, access,
correction, deletion, restriction, objection, withdrawal of consent, or a
regulatory complaint. Identity may need to be verified before fulfilling a
request.

## Children

WyrmGrid is not directed to children and does not knowingly create accounts for
or profile children. If future distribution or features make child use likely,
the privacy design and notice must be reassessed before release.

## Changes

The version date identifies this notice. WyrmGrid will require renewed review
when a material change affects what is collected, why it is used, where it is
sent, or the choices available to users.
