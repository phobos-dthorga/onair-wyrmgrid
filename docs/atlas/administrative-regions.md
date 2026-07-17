# Administrative regions in Atlas

This contract defines the sourced state, province, region, county, district,
and equivalent geography used by Atlas. These polygons provide geographical
context for planning and filtering. They are not airspace, FIR/UIR, procedure,
terrain, obstacle, or navigation-authoritative boundaries.

## Administrative levels, not US-only names

WyrmGrid stores the hierarchy as administrative levels and keeps the source's
local classification alongside it:

| WyrmGrid level | Meaning                         | Examples                                       |
| -------------- | ------------------------------- | ---------------------------------------------- |
| ADM1           | First division below a country  | state, province, region, territory, prefecture |
| ADM2           | Second division below a country | county, district, department, municipality     |

The interface should say **county** only where the source identifies a county.
It must not rename every global ADM2 feature to fit United States terminology.
Every feature keeps a stable source-derived ID, display name, optional local
name, local type, parent country, available subdivision code, administrative
level, source, and source version. Missing attributes remain unavailable; Atlas
does not invent a plausible name or classification.

## Implemented ADM1 snapshot

Regional Lens v1 bundles a processed snapshot of Natural Earth's 1:10m
Admin-1 States and Provinces data. Natural Earth describes the source as a
global first-order administrative dataset and releases its map data into the
public domain:

- <https://www.naturalearthdata.com/downloads/10m-cultural-vectors/10m-admin-1-states-provinces/>
- <https://www.naturalearthdata.com/about/terms-of-use/>

The checked-in [manifest](../../apps/desktop/static/data/atlas/admin1-manifest.json)
records the exact source commit, source and generated hashes, feature count,
licence, boundary point of view, processing tool and precision. The generated
GeoJSON is bundled with the application, so moving the pointer across the map
does not send regional lookups or viewport coordinates to Natural Earth or
GitHub.

Maintainers regenerate the snapshot explicitly from the repository root:

```powershell
npm run atlas:regions
```

The preparation script verifies the pinned source hash, performs a
topology-aware simplification with a pinned Mapshaper version, retains small
shapes, removes unused source fields, and writes stable WyrmGrid properties.
It is not part of normal development builds and does not make release builds
depend on an external download.

Natural Earth presents de-facto boundaries by default and warns that Admin-1
data is difficult to keep current. Atlas therefore shows source/version and the
contextual-only limitation in its inspector. Future source updates are reviewed
changes, never silent runtime replacements.

## Reactive regional lens

The pointer interaction uses MapLibre feature state. Geometry does not change:
literal polygon scaling would move borders, overlap neighbouring areas, and
misrepresent the source. Instead, the active region receives:

- a brighter sourced fill;
- a soft outline that widens over 160 milliseconds, creating a small lift or
  growth impression without changing the boundary;
- a transient card with local type, country, and name; and
- a persistent inspector view after click or tap.

Selection remains visually distinct from transient hover. Clicking an aircraft,
FBO, route point, or weather station takes precedence over the polygon beneath
it. Reduced-motion and `--low-resource` runs remove the animation; low-resource
mode also narrows and softens the effect while retaining the same facts.

## Semantic zoom and label priority

Atlas does not reveal every regional name at once. The bundled source includes
its own label priority and minimum useful zoom, which WyrmGrid preserves rather
than guessing importance from polygon size or treating every subdivision as
equal. Exclusive zoom bands progressively admit more of those sourced labels:

- a world or continental view shows only the highest-priority regional names;
- country-scale views introduce additional states, provinces, and equivalents;
- regional and local views reveal successively finer names; and
- the rare labels intended for extreme close-up remain hidden until that scale.

Only the active band participates in label collision detection. This is
important: merely making thousands of unwanted labels transparent would still
let them reserve screen space and suppress useful names. Within each band,
Natural Earth's `label_rank` decides which label wins when space is limited.
Names remain source-derived, and the underlying polygons stay hoverable and
selectable even when their labels are not yet shown.

Operational points remain visible at overview scale, but aircraft and FBO text
waits until closer zoom levels. Their full facts are still available through
selection and the inspector. Route and weather labels retain earlier thresholds
because those focused layers contain fewer, task-specific points. Atlas also
strengthens regional boundary contrast gradually as the camera approaches.

## ADM2 delivery gate

Global ADM2 data is materially larger and more uneven than the first-level
snapshot. It must not be added as one enormous GeoJSON document that blocks the
webview. The intended implementation is a zoom-gated, locally cached or bundled
vector-tile pack:

1. show ADM1 at regional zooms;
2. load ADM2 only at closer zooms and for intersecting tiles;
3. keep parent IDs so an ADM2 selection can resolve to its ADM1 and country;
4. expose dataset age, gaps, source, licence, and local type in the inspector;
5. allow the Compatibility profile to omit ADM2 labels or the optional pack,
   without changing ADM1 correctness; and
6. validate tile size, hostile geometry, antimeridian handling, disputed
   boundaries, render time, memory use, and offline behaviour before enabling it.

geoBoundaries `gbOpen` is the leading global ADM2 candidate because its API
publishes ADM0 through ADM5 and identifies `gbOpen` as CC BY 4.0 with
attribution. It remains a candidate until a pinned, source-year-aware tile build
and attribution design pass validation:

- <https://www.geoboundaries.org/api.html>

Country-specific authoritative sources may later replace or supplement the
global pack through versioned source adapters. Such adapters must preserve the
same WyrmGrid contract and cannot silently merge incompatible political views.
OpenStreetMap is another possible enrichment source, but its ODbL attribution
and share-alike obligations require a separate licensing and distribution
decision before its boundary data is bundled.

## Future planning joins

A pinned region becomes a safe shared filter for sourced information that
already has coordinates: airports, weather observations, jobs, fleet, staff,
cargo/passengers, and Hoard recordings. Spatial membership should be calculated
in a host-owned service when it becomes business state; a Svelte click handler
must not become the authoritative answer to "which region contains this job?"

Airspace and aviation jurisdictions remain separate layers with their own
authoritative sources and valid times. A political or administrative polygon
must never be reused as a proxy for controlled airspace.
