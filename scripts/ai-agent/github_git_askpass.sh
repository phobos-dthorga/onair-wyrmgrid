#!/usr/bin/env bash
set -euo pipefail

case "${1:-}" in
  *Username* | *username*)
    printf '%s\n' 'x-access-token'
    ;;
  *Password* | *password*)
    printf '%s\n' "${GH_TOKEN:?GH_TOKEN is required for GitHub publication.}"
    ;;
  *)
    printf '%s\n' 'Unsupported Git credential prompt.' >&2
    exit 1
    ;;
esac
