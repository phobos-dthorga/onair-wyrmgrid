#!/usr/bin/env python3
"""Sequential, bounded commissioning benchmark for local AI Agent profiles."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import pathlib
import re
import urllib.error
import urllib.request
from typing import Any


def post_chat(base_url: str, api_key: str, model: str, prompt: str) -> dict[str, Any]:
    request = urllib.request.Request(
        base_url.rstrip("/") + "/chat/completions",
        data=json.dumps(
            {
                "model": model,
                "messages": [
                    {
                        "role": "system",
                        "content": (
                            "Answer only from supplied evidence. Be concise, cite exact "
                            "paths, label uncertainty, and never request tools."
                        ),
                    },
                    {"role": "user", "content": prompt},
                ],
                "temperature": 0,
                "max_tokens": 1200,
                "reasoning_effort": "none",
                "stream": False,
            }
        ).encode("utf-8"),
        headers={
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
        },
        method="POST",
    )
    with urllib.request.urlopen(request, timeout=600) as response:
        raw = response.read(262145)
    if len(raw) > 262144:
        raise RuntimeError("Gateway response exceeded 256 KiB.")
    return json.loads(raw.decode("utf-8"))


def extract_text(response: dict[str, Any]) -> str:
    choices = response.get("choices")
    if not isinstance(choices, list) or len(choices) != 1:
        raise RuntimeError("Gateway response did not contain exactly one choice.")
    message = choices[0].get("message", {})
    text = message.get("content")
    if not isinstance(text, str) or not text.strip():
        raise RuntimeError("Gateway response content was empty.")
    return text.strip()


def run_case(
    base_url: str,
    api_key: str,
    model: str,
    case: dict[str, Any],
) -> dict[str, Any]:
    started = dt.datetime.now(dt.timezone.utc)
    try:
        response = post_chat(base_url, api_key, model, case["prompt"])
        text = extract_text(response)
        terms = {
            term: bool(re.search(re.escape(term), text, re.IGNORECASE))
            for term in case["required_terms"]
        }
        outcome = "passed" if all(terms.values()) else "failed"
        error = ""
        http_status: int | None = 200
    except urllib.error.HTTPError as exc:
        text = ""
        terms = {}
        outcome = "error"
        error = type(exc).__name__
        http_status = exc.code
    except (RuntimeError, OSError, urllib.error.URLError, json.JSONDecodeError) as exc:
        text = ""
        terms = {}
        outcome = "error"
        error = type(exc).__name__
        http_status = None
    return {
        "model": model,
        "case_id": case["id"],
        "workload": case["workload"],
        "outcome": outcome,
        "required_terms": terms,
        "response": text,
        "error_category": error,
        "http_status": http_status,
        "duration_seconds": round(
            (dt.datetime.now(dt.timezone.utc) - started).total_seconds(),
            3,
        ),
    }


def benchmark_exit_code(results: list[dict[str, Any]]) -> int:
    return 1 if any(item["outcome"] == "error" for item in results) else 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base-url", required=True)
    parser.add_argument("--credential-env", default="HOARDMIND_GATE_API_KEY")
    parser.add_argument("--cases", required=True)
    parser.add_argument("--models", nargs="+", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()

    api_key = os.environ.get(args.credential_env, "")
    if not api_key:
        raise SystemExit(f"ERROR: {args.credential_env} is empty.")
    cases = json.loads(pathlib.Path(args.cases).read_text(encoding="utf-8"))
    results: list[dict[str, Any]] = []
    for model in args.models:
        for case in cases["cases"]:
            results.append(
                run_case(args.base_url, api_key, model, case)
            )
    summary: dict[str, Any] = {}
    for workload in ("documentation", "coding"):
        summary[workload] = {}
        for model in args.models:
            selected = [
                item
                for item in results
                if item["model"] == model and item["workload"] == workload
            ]
            summary[workload][model] = {
                "passed": sum(item["outcome"] == "passed" for item in selected),
                "total": len(selected),
                "human_quality_score": None,
                "eligible_for_promotion": False,
            }
    output = {
        "schema_version": 1,
        "created_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "selection_rule": (
            "Promote only a model that passes every automated case and receives "
            "the highest operator-reviewed quality score for that workload."
        ),
        "summary": summary,
        "results": results,
    }
    destination = pathlib.Path(args.output)
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(
        json.dumps(output, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    print(f"Wrote {len(results)} sequential benchmark results to {destination}.")
    transport_errors = sum(item["outcome"] == "error" for item in results)
    if transport_errors:
        print(
            f"ERROR: {transport_errors} benchmark request(s) had transport errors.",
        )
    return benchmark_exit_code(results)


if __name__ == "__main__":
    raise SystemExit(main())
