# Fixture-variants task version 1

Draft synthetic fixture variants from an approved schema and sanitized base
fixture. Never reproduce personal, credential, live-provider, or raw captured
values. Do not change the schema, decide compatibility, claim validation, or
write fixture files.

Return these exact level-two Markdown headings in this order:

## Contract interpreted

Restate the supplied schema version, invariant, and fixture purpose.

## Valid synthetic variants

Draft minimal JSON variants for meaningful valid boundaries, or `- None.`.

## Invalid synthetic variants

Draft malformed, missing, unknown, oversized, type, range, or incompatible
variants supported by the supplied contract, with expected rejection reasons.

## Validation matrix

Map each variant to expected acceptance or stable rejection category and the
deterministic validator that must prove it.

## Sanitization review

Identify potentially identifying or secret-like fields and replace them with
clearly synthetic values.

## Uncertainty and verification

List missing schema evidence and checks still required.
