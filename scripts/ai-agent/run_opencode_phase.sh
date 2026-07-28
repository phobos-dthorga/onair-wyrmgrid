#!/usr/bin/env bash

set -euo pipefail

: "${AI_AGENT_MODEL:?AI_AGENT_MODEL is required}"
: "${AI_AGENT_PROMPT_FILE:?AI_AGENT_PROMPT_FILE is required}"
: "${AI_AGENT_EVENT_LOG:?AI_AGENT_EVENT_LOG is required}"
: "${WORKSPACE:?WORKSPACE is required}"

if [[ ! -f "$AI_AGENT_PROMPT_FILE" ]]; then
  printf 'ERROR: AI Agent prompt file is missing.\n' >&2
  exit 2
fi

prompt_sentinel=$'\x1e'
prompt="$(cat -- "$AI_AGENT_PROMPT_FILE"; printf '%s' "$prompt_sentinel")"
prompt="${prompt%$prompt_sentinel}"
opencode run \
  --format json \
  --model "$AI_AGENT_MODEL" \
  "$prompt" |
  tee -- "$WORKSPACE/$AI_AGENT_EVENT_LOG"
