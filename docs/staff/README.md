# WyrmGrid Staff

Staff is the read-only company-roster foundation for the broader WyrmGrid
operations cycle. It answers which staff facts OnAir currently reports without
assigning people, changing OnAir state, or manufacturing missing roles,
availability, or qualifications.

## Current slice

- Staff is synchronized through the same rate-protected company-data action as
  fleet, FBOs, and pending jobs.
- Successful translated snapshots are retained in the SQLCipher-encrypted Hoard
  database and restored as cached or offline data after restart.
- The roster can be searched by reported display name, airport, or
  aircraft-class qualification, then filtered by the category/status codes
  actually present, provider presence, busy-until field presence, or reported
  aircraft class. Sorting never manufactures a rank or availability state.
- The inspector shows current and home airport, busy-until time, online
  presence, numeric provider category and status, and reported class
  qualifications.
- The dossier separates overview, qualifications, and source evidence so users
  can drill down without placing every fact in one continuous panel.
- Missing fields say **Not reported**. General certifications are not treated as
  aircraft-class qualifications, and absent data is never inferred.

The screen deliberately labels category and status as numeric OnAir codes.
Although the public employee guide names common employee types and working
states, the current Swagger contract does not map those labels to the numeric
values returned by the API. Human-readable labels are deferred until that
mapping can be verified.

## Privacy and security boundary

The OnAir adapter discards fields that this slice does not need, including
salary, birth date, weight, fatigue, happiness, avatar URLs, and raw response
content. It retains the bounded `AvatarImageName` only as an opaque provider
reference because both the company roster and employee-detail responses were
verified to supply it. The reference is not treated as a path or URL and is not
sent to the webview as image source material. Only the bounded stable
`StaffSnapshot` crosses into the application service. Staff snapshots are not
exposed through the plugin protocol, optional diagnostics, or Sentry.

Portable backups include the encrypted Hoard roster because they are complete
user-requested database backups. A restored roster remains subject to the same
schema validation and is displayed as cached or offline until refreshed.

## Accessible exploration

Staff cards and fact panels can respond to fine-pointer movement with a bounded
shift and glow. The effect changes no layout or information, touch and pen input
do not trigger it, keyboard focus retains a visible static cue, and the
operating system's **Reduce Motion** preference always disables movement.

**Settings → Motion & response → Responsive information surfaces** controls the
effect and is enabled by default. Turning it off preserves every search,
filter, selection, and focus action. The reusable surface primitive can be
adopted by other workspaces, but each adoption must remain subtle and must not
turn animation into an information or authorization signal.

## Avatar compatibility decision

The official Swagger schema declares `AvatarImageName` and `AvatarUrl`. A
privacy-bounded live roster probe on 2026-07-17 observed an avatar image name on
all 89 records and no avatar URL. Five employee-detail reads produced the same
result. Swagger exposes the roster and employee-detail reads but no documented
avatar/image endpoint or URL-construction rule.

WyrmGrid therefore shows the presence of a provider reference as source
evidence but renders no portrait. It does not guess a CDN path, request an
undocumented resource, or substitute generated artwork. Actual OnAir portraits,
including the user's own avatar where present, remain blocked until OnAir
supplies an officially usable URL or documented retrieval endpoint. Enabling
that later requires URL/host validation, response-size and content-type bounds,
CSP and privacy review, caching/retention rules, and sanitized compatibility
tests.

## Later operational cycle

The Staff module is intended to participate in the planned operations journey:

1. plan and weather establish the intended flight;
2. jobs supply cargo, passengers, tourists, or company personnel;
3. Staff identifies eligible reported personnel without inventing capabilities;
4. Fleet and maintenance establish suitable available aircraft;
5. Atlas presents the planned and actual geography;
6. Hoard records the resulting evidence and debrief.

Later work may add explainable crew recommendations, duty and qualification
checks, company-personnel passenger manifests, verified avatar artwork, and industrial
workforce planning. Each additional field or write-like action requires a fresh
official API-access check, domain boundary, tests, privacy review, and explicit
authority. The public OnAir API remains read-only unless official documentation
establishes otherwise.

Official contract references:

- [OnAir public API wiki](https://onaircompany.hostwiki.io/en/Public-APIs)
- [OnAir employee guide](https://onaircompany.hostwiki.io/en/career-progression/your-employees)
- [OnAir v1 Swagger document](https://server1.onair.company/swagger/docs/v1)
