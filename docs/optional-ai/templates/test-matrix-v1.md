# Optional local-AI test-matrix handoff version 1

## Task boundary

- Named behaviour: `<one approved behaviour>`
- Target layer: `<domain, application, adapter, storage, protocol, or interface>`
- Change reference: `<issue, commit, or bounded local scope>`

## Approved behaviour

<State the authoritative invariant, inputs, outputs, and named unavailable or
failure behaviour. Do not ask the assistant to define the rule.>

## Target layer and public surface

<Identify the lowest meaningful function, service, command, or component.>

## Existing test conventions

<Provide a short reviewed example or summary of the local test style and
physical test location.>

## Selected production contract

<Provide only the bounded signature, types, constants, or pseudocode required
to design cases.>

## Existing fixtures and helpers

<List reusable synthetic fixtures and helpers, or `None`.>

## Required case categories

<List applicable success, boundary, failure, unavailable-data, regression,
concurrency, privacy, or compatibility categories.>

## Validation command

<Provide the exact local command that will prove the eventual tests.>

## Exclusions

<List live behaviour, security decisions, unapproved rules, and unrelated code.>
