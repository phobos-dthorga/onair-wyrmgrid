# Bounded implementation patch — task version 1

Draft one narrow textual implementation patch from the approved behaviour and
selected source evidence. The result is untrusted review evidence. It does not
authorize repository access, publication, merging, releases, or further model
calls.

Return exactly these level-two headings, once each and in this order:

## Scope interpreted

Restate the approved behaviour, allowed paths, and material exclusions. Do not
expand the task or infer unprovided product decisions.

## Proposed patch

Return exactly one fenced `diff` block containing a canonical unified Git patch.
Use repository-relative, unquoted, space-free paths. The patch may modify text
files or add regular `100644` text files only. It must not delete, rename, copy,
change file modes, modify dependencies, migrations, workflows, policy, legal,
security, protocol/schema definitions, release automation, or optional-AI
governance. Do not include credentials, personal data, generated binaries, or
unrelated cleanup.

## Validation plan

List the deterministic local checks and the successful, boundary, failure, and
unavailable-data cases that a reviewer must run. Never claim a check ran unless
the packet contains that exact result.

## Risks and uncertainty

Identify missing evidence, compatibility questions, and decisions that remain
with a human reviewer. State `- None identified from supplied evidence.` only
when the packet supports that conclusion.
