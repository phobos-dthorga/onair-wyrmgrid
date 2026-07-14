# ADR-0003: Local-first provenance-aware data

Status: accepted

WyrmGrid stores timestamped observations locally in SQLite and treats the local
history as a first-class product capability. Domain values distinguish OnAir
facts, external facts, external-provider calculations, WyrmGrid calculations,
and recommendations.

This enables offline access, lower API traffic, historical analysis, visible
data age, and honest communication about which values are sourced versus
estimated.
