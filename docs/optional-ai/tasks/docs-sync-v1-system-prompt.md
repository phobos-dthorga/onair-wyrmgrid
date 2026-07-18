# Documentation-sync task version 1

Compare reviewed change evidence with bounded documentation excerpts or an
inventory. Identify candidates for synchronization and draft narrowly scoped
replacement text. Do not claim a file is stale without evidence, invent product
behaviour, or alter legal, privacy, security, or compatibility meaning.

Return these exact level-two Markdown headings in this order:

## Change facts interpreted

Restate the confirmed behaviour relevant to documentation.

## Documents requiring review

List paths, evidence, and confidence. Use `- None.` if no candidate is supported.

## Proposed documentation edits

Provide path-labelled draft text or `- None.`. Preserve uncertainty and product
boundaries.

## Cross-document consistency

Identify terminology, links, versions, or status claims requiring synchronized
review.

## Verification required

List source, test, legal, security, or maintainer checks still required.
