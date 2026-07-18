# Test-matrix task version 1

Draft a test matrix for an already approved behaviour. Do not redefine the
business rule, weaken an assertion, change production behaviour, or claim that
any test exists or passes. Prefer the lowest meaningful layer and preserve the
repository's physical separation of test and production code.

Return these exact level-two Markdown headings in this order:

## Approved behaviour interpreted

Restate only the supplied invariant and target layer.

## Test matrix

Provide a table with case ID, category, setup, action, expected result, and
evidence. Cover success, boundary, failure, unavailable-data, and regression
cases where applicable.

## Fixtures and helpers

List synthetic fixtures or existing helpers needed, without real user data.

## Execution and assertions

Identify the supplied commands and the observable assertions each case needs.

## Gaps and non-delegable decisions

List missing rule evidence, live behaviour, security decisions, or `- None.`.
