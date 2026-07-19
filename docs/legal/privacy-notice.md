# OnAir WyrmGrid Privacy Notice

**Version and effective date:** 2026-07-19.2 (persisted Atlas and host-owned
plugin preferences)

This preliminary notice describes information handled by official builds of
OnAir WyrmGrid distributed by Phobos A. D'thorga. It does not describe an
unofficial fork, community plugin, or a third party's own service. WyrmGrid is
still in foundation-stage development. This notice must be updated and
professionally reviewed before a stable or commercial release.

## Privacy at a glance

- WyrmGrid is local-first and has no WyrmGrid user account or advertising.
- WyrmGrid's local database is encrypted with SQLCipher. Its random key is held
  separately by the operating-system credential service.
- Portable backups are created only when you request one, are encrypted with a
  password you choose, and remain wherever you place them until you or your
  storage provider deletes them.
- Your OnAir API key is used for the active connection and is not written to the
  WyrmGrid database. If you explicitly ask WyrmGrid to remember it, the key is
  stored separately by the operating-system credential service.
- Staff synchronization may retrieve display names, provider category and
  status codes, current and home airports, availability times, online presence,
  reported aircraft-class qualifications, and an opaque provider avatar-image
  reference. WyrmGrid stores only this bounded translated roster locally; the
  public API currently supplies no usable avatar URL, so WyrmGrid does not load
  or invent portrait artwork. Raw responses and unneeded personal or employment
  fields are excluded.
- If you choose to import a flight plan, your SimBrief Pilot ID or username is
  sent directly to SimBrief to retrieve the account's latest OFP. WyrmGrid does
  not ask for a SimBrief or Navigraph password. The reference is remembered in
  the encrypted local database only if you select that option.
- If you explicitly begin a flight operation, WyrmGrid stores the sanitized
  imported plan, the selected read-only OnAir job observation and originating
  company identifier when present, and a per-leg aggregate manifest in the
  encrypted local database. Missing passenger counts or freight weights remain
  marked unavailable.
- If you choose to fetch airport weather, the plan's origin, destination, and
  alternate ICAO identifiers are sent to AviationWeather.gov for current METAR
  and TAF products.
- If you approve and start the optional Open-Meteo or RainViewer provider,
  WyrmGrid requests a coarse global model grid or a small current global radar
  tile set. These requests contain host-selected public coordinates or tile
  addresses, not an OnAir company, SimBrief account, or flight plan.
- Simulator telemetry is stored only after you explicitly start a recording or
  opt into automatic recording. These local sessions can contain precise
  routes, flight times, measurements, lifecycle events, and an associated
  sanitized SimBrief plan, and can be deleted from WyrmGrid.
- Atlas downloads its current map style and tiles from MapLibre's public demo
  service after legal onboarding is complete.
- Privacy-filtered error diagnostics are optional, off by default, and only
  operate when both the user and the official build enable Sentry.
- Imported community language packs, their message text, and optional author
  metadata remain local and are not sent to translation services.
- A plugin can start on future WyrmGrid launches only when you separately choose
  automatic start after granting it standing access. The choice is stored in the
  encrypted local database and is invalidated by material plugin-scope changes.
- Atlas layer visibility and the automatic synchronization interval are stored
  in the encrypted local database. If you opt into **Restore my last Atlas
  view**, WyrmGrid also stores the last map centre, zoom, bearing, and pitch on
  this device; turning the option off deletes those saved camera values.
- Host-owned plugin settings use bounded, non-secret choices stored in the
  encrypted local database. Plugins cannot read or change those records
  directly.
- **Erase the WyrmGrid database** can replace the active encrypted database with
  an empty one after an acknowledgement, exact typed phrase, and restart. It
  does not erase portable backups, installed plugins, diagnostics, simulator
  sidecars, browser-webview local storage, or credentials held separately by
  the operating system.
- WyrmGrid does not sell personal information.

## Information kept locally

WyrmGrid currently keeps the following information on the user's device:

- the accepted Terms and Privacy Notice versions, the optional diagnostics
  preference, and acknowledgement timestamps in WyrmGrid's SQLCipher-encrypted
  SQLite database;
- the selected theme and language pack, imported custom theme manifests, and
  imported community language-pack manifests in WyrmGrid's encrypted database. A
  language-pack manifest includes its translated text and may include the
  author name supplied by the pack creator;
- interface preferences, including the selected automatic synchronization
  interval, Atlas layer visibility, responsive-surface motion choice, and an
  optional last Atlas map centre, zoom, bearing, and pitch, in WyrmGrid's
  encrypted database. Last-view values are retained only while restoration is
  enabled. An older webview synchronization choice is migrated once and removed
  from webview storage after the encrypted save succeeds; and
- while the application is running, the supplied OnAir company ID, API key,
  translated company details, and current fleet, FBO, pending-job, and staff
  observations in process memory. If the user selects **Remember this
  connection**, Windows' credential service stores the API key while the
  SQLCipher database stores the Company ID and separate default-off automatic
  connection choice;
- successful translated fleet, FBO, pending-job, and staff observations in WyrmGrid's
  local Hoard database. These records contain stable WyrmGrid fields and
  provenance, not the raw OnAir response or API key. Staff records are limited
  to display name, numeric provider category and status, current and home
  airports, busy-until time, online presence, and reported aircraft-class
  qualifications, plus an opaque bounded avatar image-name reference. The
  reference is not used as a URL or image path. WyrmGrid does not retain salary,
  birth date, weight, fatigue, happiness, avatar URLs or artwork, or other unused
  employee fields;
- after the user explicitly starts recording or enables automatic recording,
  translated simulator session identity and one-hertz position, on-ground,
  engine, parking-brake, pause, altitude, speed, fuel, weight, attitude,
  simulator-time, and observation-time facts in the local WyrmGrid database.
  Raw SimConnect messages are not persisted;
- when a current imported plan is associated with a recording, its sanitized
  validated `FlightPlanSnapshot`, plan/recording correlation version, and
  calculated comparisons. The account reference is not copied into that
  recording, and the raw OFP response is not written to the database;
- after the user explicitly begins a flight operation, a random local operation
  identifier and immutable numbered revisions containing the sanitized plan,
  selected validated OnAir job observation and originating company identifier
  when present, and deterministic per-leg aggregate passenger counts or freight
  weights. Missing source fields are retained as unavailable; WyrmGrid does not
  add individual people, consignments, or assumed loads;
- symbolic authorization grant and revoke decisions, including actor ID,
  capability scope revision, capability count, and decision time. These records
  are limited to the newest 4,096 decisions and never contain API keys, raw
  plugin output, or simulator payloads;
- a host-owned per-plugin automatic-start choice, when enabled. It is associated
  with the plugin identifier and exact reviewed scope and is removed when access
  is revoked;
- host-owned, bounded, non-secret per-plugin configuration values, such as
  weather refresh cadence. Plugin processes cannot access or write these
  records;
- a random 32-byte database key in the operating-system credential service,
  identified by WyrmGrid's application service and key-version label. The key
  is not stored in the database or portable backups; and
- a user-supplied SimBrief Pilot ID or username for the duration of an import
  request and the translated latest OFP in process memory. If **Remember this
  account reference** is selected after a successful import, that Pilot ID or
  username is stored in the SQLCipher database; the translated plan is written
  only when associated with a local recording or explicitly accepted as a
  flight-operation revision; and
- after an explicit weather request, translated METAR and TAF observations for
  at most ten plan airports in process memory. The current cache is tied to the
  session plan, reused for ten minutes, and not written to the WyrmGrid database.

The active API key is cleared from WyrmGrid's connection state when the OnAir
session disconnects or the process exits. A separately remembered Windows
credential remains until **Forget saved details** succeeds or the user removes
it through the operating system. Normal secret handling cannot guarantee
removal from operating-system crash dumps, virtual memory, or a compromised
computer.

When the user creates or restores a portable backup, WyrmGrid briefly handles
the selected local path and supplied backup password in process memory. It does
not persist the password or send either value to WyrmGrid, Sentry, plugins, or
an external service. The chosen backup is a complete encrypted copy of local
database content, including remembered Company ID, automatic-connect choice,
SimBrief account reference, accepted flight operations, and their retained
revisions, but it cannot contain the OnAir API key held by the operating system.
Restore creates encrypted pending and rollback files beside the active database
until the next successful startup completes activation.

## Connections to other services

Selecting or importing a language pack does not contact a translation service.
WyrmGrid validates the supplied file locally and does not send its messages or
author metadata to Sentry, external providers, or plugins.

### OnAir

When the user chooses to connect—or separately enables automatic connection on
startup—WyrmGrid sends the company ID and API key directly to OnAir's public API
and requests the selected company information. Automatic connection is off by
default and begins only after current legal acknowledgement.
Subsequent synchronization requests retrieve fleet, FBO-network, pending-job,
and bounded staff-roster information. OnAir operates
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

### SimBrief

When the user explicitly chooses **Import latest OFP**, WyrmGrid sends the
entered Pilot ID or username and a request for JSON directly to SimBrief's
latest-OFP endpoint over HTTPS. SimBrief and its network providers receive that
identifier, the source IP address, request time, user-agent information, and
other normal connection metadata. A successful response may contain private
operational information including airports, route, times, aircraft identity,
weights, fuel, alternates, coordinates, plan identifiers, and AIRAC details.

WyrmGrid translates allowlisted fields into a local `FlightPlanSnapshot` and
does not send the identifier, raw response, or translated plan to Sentry or
plugins. Clearing the plan or closing WyrmGrid removes the current in-memory
reference, subject to the same operating-system memory limitations. A Pilot ID
or username explicitly remembered by the user remains as encrypted account
metadata until a successful import with remembering cleared, or local data is
deleted.
If a simulator recording is active, or starts while that plan remains current,
WyrmGrid retains the sanitized snapshot with the encrypted recording so its
planned and recorded facts can be reviewed later. Clearing Dispatch does not
rewrite an already associated recording. SimBrief operates independently under
its own terms and privacy practices.

### AviationWeather.gov

When the user has approved and started the AviationWeather.gov provider and
explicitly chooses **Fetch airport weather**, WyrmGrid sends a
bounded list containing the imported plan's origin, destination, and alternate
ICAO identifiers to AviationWeather.gov's public METAR and TAF JSON endpoints
over HTTPS. AviationWeather.gov and intermediate network providers receive the
station identifiers, source IP address, request time, WyrmGrid user agent, and
other normal connection metadata. WyrmGrid does not include the SimBrief Pilot
ID or username, SimBrief plan identifier, route, OnAir API key, company ID,
fleet record, or other OFP fields in these requests.

Raw provider JSON remains inside the provider plugin. WyrmGrid retains only a
bounded translated `WeatherSnapshot` in process memory, visibly identifies the
source and product times, and reuses a successful combined airport result for
ten minutes to reduce provider load. Only this approved provider plugin receives
the station list; weather is not sent to Sentry or other plugins.
The provider may return no report for a valid station; absence of data is not a
claim of safe or clear conditions. AviationWeather.gov is operated by the
United States National Weather Service and operates independently under its own
notices and policies.

### Open-Meteo and RainViewer

When the user approves and starts these independent provider plugins,
Open-Meteo receives a coarse set of 84 global latitude/longitude samples about
every fifteen minutes, and RainViewer receives a metadata request plus four
zoom-one global tile requests about every five minutes. The selections are made
by WyrmGrid and do not contain a user-entered route, account reference, OnAir
fact, simulator telemetry, or credential. The services and intermediate network
providers receive the source IP address, request time, WyrmGrid provider user
agent, and other ordinary connection metadata.

Raw responses remain inside the corresponding provider process. Open-Meteo
publishes bounded numeric samples; RainViewer publishes validated PNG bytes.
WyrmGrid retains only the most recent valid in-memory layer and removes the
active contribution when the plugin stops. Neither provider is contacted until
its requested capabilities are approved and it is started in Forge, or the user
has separately approved standing access and enabled automatic start. Automatic
start is off by default and runs only after current legal acknowledgement.

### Optional Sentry diagnostics

WyrmGrid also keeps a separate local diagnostic log whether or not optional
Sentry telemetry is enabled. It contains timestamps, severity, stable error
codes, operation names, and controlled English messages. It is designed not to
contain OnAir credentials, company identifiers, raw provider responses, local
paths, plugin output, or user-entered text. The log retains at most 200 entries,
can be reviewed and cleared from **Diagnostics**, and is never uploaded or
attached automatically.

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
retrieve a user-requested SimBrief plan and optional airport or global weather, compare separately
sourced operational facts, display operational context, remember local choices,
secure and diagnose the application, and improve reliability.
Information is not used for behavioural advertising, data brokerage, or
unrelated user profiling.

## Retention and deletion

Session-only credentials, unremembered account references, fleet state,
SimBrief plans that were not accepted into an operation or associated with a
recording, weather snapshots, and global weather layers are discarded when the
process exits. A
Dispatch user can also clear the current plan and its associated weather during
the session. A plan already associated with a recording follows that recording's
retention and deletion instead.
Remembered provider metadata remains until the user clears the relevant
remembering choice, uses **Forget saved details**, or deletes the application's
local data. The OnAir key stored by Windows is outside portable backups. Local
preferences and imported customisation manifests remain until changed, removed
by a management function, or deleted with the application's local data.
Version 1 does not yet provide an individual language-pack deletion control.
The **Erase the WyrmGrid database** control removes the active, pending, and
rollback SQLite databases during restart and creates a new empty encrypted
database. This deletes every SQLite-held record and preference, including plugin
permissions, automatic-start choices, legal acknowledgements, and telemetry
consent. It deliberately leaves installed files, diagnostics, portable backups,
browser-webview local storage, and operating-system credentials in place; those
have their own deletion controls or must be removed separately. Filesystem
snapshots and recovery tools may still retain deleted data, so this is not
forensic secure erasure.
Accepted flight operations and their immutable revisions remain until the local
database is deleted or a future operation-management control removes them; the
current foundation does not yet offer per-operation deletion.
The local diagnostic log rotates at 200 entries and can be cleared from the
application. Optional Sentry diagnostic events follow
the Sentry retention configuration disclosed when public telemetry is enabled.
Completed and interrupted simulator recordings use the retention period chosen
in Settings (30 days by default). A user can pin a recording against automatic
pruning, or delete recordings individually or together; active recordings must
first be stopped. Retention pruning is local and does not send session content
to WyrmGrid, Sentry, simulator providers, or plugins. JSON and CSV exports are
plaintext copies created only on request and are no longer protected by the
database once saved elsewhere.
Uninstallers and operating systems may not remove every application-data file;
users can request instructions for locating it.

Portable backups are not managed by WyrmGrid after creation. They remain at the
user-selected location and may also be retained by synchronisation providers,
system backups, snapshots, removable media, or deleted-file recovery. Removing
local application data does not delete those copies. WyrmGrid cannot recover a
forgotten backup password or an encrypted database whose operating-system key
and usable portable backups have both been lost.

## Security and limits

WyrmGrid minimises diagnostic fields, keeps credentials out of plugins, uses
encrypted HTTPS connections to external services, encrypts its persistent
database and portable backups, and treats remote data as untrusted. Encryption
at rest does not protect data already opened by the running application, a
compromised operating-system account, process memory, crash dumps, screenshots,
or a disclosed backup password. No system can promise absolute security. The
threat model documents known boundaries and remaining work.

## Choices and requests

Users can decline optional diagnostics without losing the core application,
change that preference later, disconnect OnAir, forget a remembered OnAir
credential, stop remembering a SimBrief account reference, create an encrypted
portable backup, restore it on another installation, and delete local
application data.
Questions or requests can be raised through the project repository. Use
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
