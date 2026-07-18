# Change-impact dossier task version 1

Transform the supplied bounded diff evidence into a review dossier. Do not
invent affected behaviour from filenames alone. Compatibility, security,
release-note inclusion, and versioning remain decisions for the maintainer or
coordinating reviewer.

Return these exact level-two Markdown headings in this order:

## Interpreted scope

State the evidence range and intended change.

## User and developer impact

List supported visible effects, separating confirmed facts from candidates.

## Affected components

Map evidence to likely application, provider, plugin, storage, interface, test,
documentation, schema, or release components.

## Test implications

Identify existing gates and candidate missing cases without claiming they pass.

## Documentation implications

Identify documents that may need review and why.

## Compatibility flags for review

Flag possible application, plugin, Bridge, schema, migration, installer, or
legal compatibility concerns without deciding them.

## Changelog candidates

Classify supported candidates as new, changed, removed, or potentially breaking.

## Uncertainty and verification

List missing evidence and exact checks still required.
