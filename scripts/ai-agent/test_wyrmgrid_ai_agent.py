from __future__ import annotations

import argparse
import importlib.util
import io
import json
import pathlib
import subprocess
import sys
import tempfile
import unittest
import urllib.error
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[2]
POLICY_PATH = ROOT / "ci" / "ai-agent-policy.yml"


def load_local_module(name: str, path: pathlib.Path):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Unable to load test module: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


benchmark = load_local_module(
    "wyrmgrid_ai_agent_benchmark",
    ROOT / "scripts" / "ai-agent" / "benchmark_models.py",
)
agent = load_local_module(
    "wyrmgrid_ai_agent_runtime",
    ROOT / "scripts" / "ai-agent" / "wyrmgrid_ai_agent.py",
)


class WyrmGridAiAgentTests(unittest.TestCase):
    def setUp(self) -> None:
        self.policy = agent.load_policy(POLICY_PATH)

    def test_policy_is_json_compatible_yaml_with_six_modes(self) -> None:
        modes = self.policy["modes"]
        self.assertEqual(4, len(modes["read_only"]))
        self.assertEqual(["PATCH", "FEATURE"], modes["change"])
        self.assertEqual(20, self.policy["job"]["retention_builds"])
        self.assertEqual(30, self.policy["job"]["retention_days"])
        self.assertEqual(
            ["LOW", "MEDIUM", "HIGH"],
            self.policy["job"]["local_reasoning_efforts"],
        )

    def test_policy_exposes_editable_context_profiles(self) -> None:
        profiles = self.policy["model_profiles"]
        self.assertEqual(
            "qwen3.6:35b",
            profiles["REPOSITORY_SCHOLAR_LOCAL"]["selected_model"],
        )
        self.assertEqual(
            "qwen3-coder:30b",
            profiles["SCOPED_BUILDER_LOCAL"]["selected_model"],
        )
        self.assertEqual(
            "SMALL_FILES",
            self.policy["context_limits"]["active_profile"],
        )
        limits = agent.active_context_limits(self.policy)
        self.assertEqual(32768, limits["maximum_visible_file_bytes"])
        self.assertEqual(800, limits["maximum_visible_file_lines"])
        self.assertEqual(524288, limits["maximum_visible_total_bytes"])

    def test_validate_toolchain_records_only_pinned_versions(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            artifact_dir = pathlib.Path(temp)
            args = argparse.Namespace(
                policy=str(POLICY_PATH),
                artifact_dir=str(artifact_dir),
            )
            outputs = {
                "opencode": subprocess.CompletedProcess(
                    ["opencode", "--version"], 0, "1.18.5\n", ""
                ),
                "codex": subprocess.CompletedProcess(
                    ["codex", "--version"], 0, "codex-cli 0.145.0\n", ""
                ),
            }
            with (
                mock.patch.object(agent.shutil, "which", return_value="/bin/tool"),
                mock.patch.object(
                    agent.subprocess,
                    "run",
                    side_effect=lambda command, **_: outputs[command[0]],
                ),
            ):
                agent.validate_toolchain(args)

            evidence = json.loads(
                (artifact_dir / "toolchain.json").read_text(encoding="utf-8")
            )
            self.assertEqual(
                "Jenkins AI Agent plugin", evidence["execution_interface"]
            )
            self.assertEqual(
                "1.18.5", evidence["tools"]["opencode"]["reported_version"]
            )
            self.assertEqual(
                "0.145.0", evidence["tools"]["codex"]["reported_version"]
            )

    def test_benchmark_records_http_status_without_error_body(self) -> None:
        failure = urllib.error.HTTPError(
            "https://gateway.invalid/v1/chat/completions",
            403,
            "Forbidden",
            hdrs=None,
            fp=io.BytesIO(b"private upstream detail"),
        )
        case = {
            "id": "transport-error",
            "workload": "documentation",
            "prompt": "Use supplied evidence.",
            "required_terms": ["evidence"],
        }
        with mock.patch.object(benchmark, "post_chat", side_effect=failure):
            result = benchmark.run_case(
                "https://gateway.invalid/v1",
                "credential-not-logged",
                "local-model",
                case,
            )
        failure.close()

        self.assertEqual("error", result["outcome"])
        self.assertEqual("HTTPError", result["error_category"])
        self.assertEqual(403, result["http_status"])
        self.assertEqual("", result["response"])
        self.assertEqual(1, benchmark.benchmark_exit_code([result]))

    def test_benchmark_disables_reasoning_for_bounded_final_answers(self) -> None:
        response = mock.MagicMock()
        response.__enter__.return_value = response
        response.read.return_value = (
            b'{"choices":[{"message":{"content":"bounded answer"}}]}'
        )

        with mock.patch.object(
            benchmark.urllib.request,
            "urlopen",
            return_value=response,
        ) as urlopen:
            benchmark.post_chat(
                "https://gateway.invalid/v1",
                "credential-not-logged",
                "local-model",
                "Use supplied evidence.",
            )

        request = urlopen.call_args.args[0]
        body = json.loads(request.data.decode("utf-8"))
        self.assertEqual("none", body["reasoning_effort"])
        self.assertEqual(1200, body["max_tokens"])
        self.assertFalse(body["stream"])

    def test_repair_prompt_retains_prior_failures_as_regression_constraints(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            artifacts = pathlib.Path(temporary)
            tests = artifacts / "tests"
            tests.mkdir()
            (artifacts / "parameters.json").write_text(
                json.dumps({"request": "Repair the scoped path guard."}),
                encoding="utf-8",
            )
            (tests / "output.txt").write_text(
                "FAIL: wildcard-all was accepted\n",
                encoding="utf-8",
            )

            agent.repair_prompt(
                argparse.Namespace(artifact_dir=str(artifacts), attempt=1)
            )
            (tests / "output.txt").write_text(
                "FAIL: mixed separators evaded traversal detection\n",
                encoding="utf-8",
            )
            agent.repair_prompt(
                argparse.Namespace(artifact_dir=str(artifacts), attempt=2)
            )

            prompt = (artifacts / "repair-prompt-2.md").read_text(
                encoding="utf-8"
            )
            history = json.loads(
                (artifacts / "repair-failure-history.json").read_text(
                    encoding="utf-8"
                )
            )
            self.assertIn("Earlier failures", prompt)
            self.assertIn("regression constraints", prompt)
            self.assertIn("wildcard-all was accepted", prompt)
            self.assertIn("mixed separators evaded traversal detection", prompt)
            self.assertEqual([1, 2], [item["attempt"] for item in history])

    def test_read_only_repair_prompt_retains_citation_failures(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            artifacts = pathlib.Path(temporary)
            (artifacts / "parameters.json").write_text(
                json.dumps(
                    {
                        "mode": "ASK",
                        "request": "Explain the source and embedding boundary.",
                    }
                ),
                encoding="utf-8",
            )
            first_failure = artifacts / "validation-attempt-0.log"
            first_failure.write_text(
                "Citation was not covered by a completed read: docs/roadmap.md:24-26\n",
                encoding="utf-8",
            )
            agent.read_only_repair_prompt(
                argparse.Namespace(
                    artifact_dir=str(artifacts),
                    attempt=1,
                    failure_log=str(first_failure),
                )
            )
            second_failure = artifacts / "validation-attempt-1.log"
            second_failure.write_text(
                "Cited source lines do not contain distinctive answer terms.\n",
                encoding="utf-8",
            )
            agent.read_only_repair_prompt(
                argparse.Namespace(
                    artifact_dir=str(artifacts),
                    attempt=2,
                    failure_log=str(second_failure),
                )
            )

            prompt = (
                artifacts / "read-only-repair-prompt-2.md"
            ).read_text(encoding="utf-8")
            history = json.loads(
                (
                    artifacts / "read-only-failure-history.json"
                ).read_text(encoding="utf-8")
            )
            self.assertIn("Re-read every exact source range", prompt)
            self.assertIn("Earlier failures", prompt)
            self.assertIn("constraints and must remain corrected", prompt)
            self.assertIn("docs/roadmap.md:24-26", prompt)
            self.assertIn("distinctive answer terms", prompt)
            self.assertEqual([1, 2], [item["attempt"] for item in history])

    def test_change_repair_prompt_retains_pretest_failures(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            artifacts = pathlib.Path(temporary)
            (artifacts / "parameters.json").write_text(
                json.dumps(
                    {
                        "mode": "PATCH",
                        "request": "Clarify the README index.",
                    }
                ),
                encoding="utf-8",
            )
            first_failure = artifacts / "validation-attempt-0.log"
            first_failure.write_text(
                "Agent response did not end with a Citations section.\n",
                encoding="utf-8",
            )
            agent.change_repair_prompt(
                argparse.Namespace(
                    artifact_dir=str(artifacts),
                    attempt=1,
                    failure_log=str(first_failure),
                )
            )
            second_failure = artifacts / "validation-attempt-1.log"
            second_failure.write_text(
                "Change mode produced no tracked repository diff.\n",
                encoding="utf-8",
            )
            agent.change_repair_prompt(
                argparse.Namespace(
                    artifact_dir=str(artifacts),
                    attempt=2,
                    failure_log=str(second_failure),
                )
            )

            prompt = (
                artifacts / "change-repair-prompt-2.md"
            ).read_text(encoding="utf-8")
            history = json.loads(
                (artifacts / "change-failure-history.json").read_text(
                    encoding="utf-8"
                )
            )
            self.assertIn("Printing pseudo-calls", prompt)
            self.assertIn("<function=read>", prompt)
            self.assertIn("Earlier failures", prompt)
            self.assertIn("did not end with a Citations section", prompt)
            self.assertIn("produced no tracked repository diff", prompt)
            self.assertEqual([1, 2], [item["attempt"] for item in history])

    def test_benchmark_accepts_completed_low_quality_result_as_evidence(self) -> None:
        result = {
            "outcome": "failed",
            "http_status": 200,
        }
        self.assertEqual(0, benchmark.benchmark_exit_code([result]))

    def test_normalize_path_rejects_unbounded_and_traversal_scopes(self) -> None:
        forbidden = set(self.policy["scope"]["forbidden_roots"])
        for value in ("", ".", "/", "*", "**", "../docs", "docs/../../secret", ".git"):
            with self.subTest(value=value), self.assertRaises(agent.PolicyError):
                agent.normalize_relative_path(value, forbidden)

    def test_scope_matching_is_prefix_bounded(self) -> None:
        self.assertTrue(agent.is_within_scope("docs/roadmap.md", ["docs"]))
        self.assertTrue(agent.is_within_scope("Jenkinsfile.ai-agent", ["Jenkinsfile.ai-agent"]))
        self.assertFalse(agent.is_within_scope("docs-private/secret.md", ["docs"]))

    def test_auto_model_profile_depends_on_mode(self) -> None:
        self.assertEqual(
            "REPOSITORY_SCHOLAR_LOCAL", agent.choose_model_profile("ASK", "AUTO")
        )
        self.assertEqual(
            "SCOPED_BUILDER_LOCAL", agent.choose_model_profile("FEATURE", "AUTO")
        )

    def test_opencode_config_denies_shell_web_and_out_of_scope_edits(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            artifacts = root / "ci-artifacts" / "ai-agent"
            artifacts.mkdir(parents=True)
            parameters = {
                "mode": "FEATURE",
                "allowed_paths": ["docs", "scripts/ci/example.py"],
                "reasoning_effort": "LOW",
            }
            config = agent.build_opencode_config(
                root, artifacts, parameters, self.policy
            )
        permission = config["permission"]
        self.assertEqual("deny", permission["bash"])
        self.assertEqual("deny", permission["webfetch"])
        self.assertEqual("deny", permission["external_directory"])
        self.assertEqual("deny", permission["edit"]["*"])
        self.assertEqual("allow", permission["edit"]["docs/**"])
        self.assertEqual(
            self.policy["job"]["agent_steps"]["FEATURE"],
            config["agent"]["build"]["steps"],
        )
        self.assertEqual(0, config["agent"]["build"]["temperature"])
        self.assertEqual(
            "{file:./agent-system.md}", config["agent"]["build"]["prompt"]
        )
        self.assertTrue(config["agent"]["title"]["disable"])
        for model in config["provider"]["hoardmind-gate"]["models"].values():
            self.assertEqual("low", model["options"]["reasoningEffort"])
            self.assertEqual(12288, model["limit"]["context"])
            self.assertEqual(1200, model["limit"]["output"])

    def test_opencode_config_rejects_unregistered_reasoning_effort(self) -> None:
        parameters = {
            "mode": "ASK",
            "allowed_paths": [],
            "reasoning_effort": "EXTREME",
        }
        with self.assertRaisesRegex(
            agent.PolicyError, "Unsupported REASONING_EFFORT"
        ):
            agent.build_opencode_config(
                ROOT,
                ROOT / "ci-artifacts" / "ai-agent",
                parameters,
                self.policy,
            )

    def test_read_only_system_prompt_forbids_unrequested_summary(self) -> None:
        prompt = agent.render_agent_system_prompt("ASK")
        self.assertIn("machine-validated", prompt)
        self.assertIn("Never summarize a whole file", prompt)
        self.assertIn("records its immutable revision, hash, and line", prompt)
        self.assertIn("Before citing\nany other file, you MUST make", prompt)
        self.assertIn("never invent, translate, abbreviate, uppercase", prompt)
        self.assertIn("numbered immutable root guidance", prompt)
        self.assertIn("not literally\npresent there", prompt)
        self.assertIn("Citations:", prompt)
        self.assertIn("final section", prompt)

    def test_change_system_prompt_preserves_jenkins_test_authority(self) -> None:
        prompt = agent.render_agent_system_prompt("FEATURE")
        self.assertIn("permitted edit tools", prompt)
        self.assertIn("Jenkins does that", prompt)
        self.assertIn("Citations:", prompt)

    def test_system_prompt_numbers_the_hash_bound_root_guidance(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            repository = pathlib.Path(temporary)
            (repository / "AGENTS.md").write_text(
                "# Guidance\n\nRust application services own WyrmGrid business rules.\n",
                encoding="utf-8",
            )
            evidence = agent.record_injected_guidance_evidence(
                repository, "a" * 40
            )
            context = agent.render_numbered_root_guidance(repository, evidence)
            prompt = agent.render_agent_system_prompt("ASK", context)
            self.assertIn('path="AGENTS.md"', prompt)
            self.assertIn(evidence["files"][0]["sha256"], prompt)
            self.assertIn("1 | # Guidance", prompt)
            self.assertIn(
                "3 | Rust application services own WyrmGrid business rules.", prompt
            )

    def test_prompt_delegates_artifacts_to_jenkins_and_requires_citations(self) -> None:
        prompt = agent.render_prompt(
            {
                "mode": "ASK",
                "source_revision": "a" * 40,
                "request": "Answer from the repository.",
                "allowed_paths": [],
                "local_model_profile": "REPOSITORY_SCHOLAR_LOCAL",
            }
        )
        self.assertIn("Jenkins captures your final response", prompt)
        self.assertIn("not create or edit `.agent-output`", prompt)
        self.assertIn("file range more than", prompt)
        self.assertIn("path:start-end", prompt)
        self.assertIn("nonempty answer", prompt)
        self.assertIn("never synthesize an alias", prompt)
        self.assertIn("answer directly without reading", prompt)
        self.assertIn("the inventory\nis an index", prompt)
        self.assertNotIn("Local profile:", prompt)
        self.assertNotIn("docs/example.md", prompt)

    def test_answer_must_precede_final_citations(self) -> None:
        self.assertEqual(
            "Rust application services own WyrmGrid business rules.",
            agent.answer_from_text(
                "Rust application services own WyrmGrid business rules.\n\n"
                "Citations:\n- AGENTS.md:19-19"
            ),
        )
        with self.assertRaises(agent.PolicyError):
            agent.answer_from_text("Citations:\n- AGENTS.md:19-19")
        with self.assertRaises(agent.PolicyError):
            agent.answer_from_text(
                "Rust application services own WyrmGrid business rules.\n\n"
                "Citations:\n- AGENTS.md:19-19\nextra text"
            )

    def test_absolute_citations_are_canonicalized_only_inside_worktree(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            repository = pathlib.Path(temporary) / "repository"
            repository.mkdir()
            guidance = repository / "AGENTS.md"
            guidance.write_text("# Guidance\n", encoding="utf-8")
            normalized = agent.normalize_citation_paths(
                [
                    {
                        "path": str(guidance),
                        "line_start": 1,
                        "line_end": 1,
                    }
                ],
                repository,
            )
            self.assertEqual(
                [{"path": "AGENTS.md", "line_start": 1, "line_end": 1}],
                normalized,
            )
            with self.assertRaises(agent.PolicyError):
                agent.normalize_citation_paths(
                    [
                        {
                            "path": str(repository.parent / "outside.md"),
                            "line_start": 1,
                            "line_end": 1,
                        }
                    ],
                    repository,
                )

    def test_jenkins_captures_final_text_and_exact_citations(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            repository = root / "repository"
            repository.mkdir()
            subprocess.run(["git", "init", "-q"], cwd=repository, check=True)
            subprocess.run(
                ["git", "config", "user.email", "test@example.invalid"],
                cwd=repository,
                check=True,
            )
            subprocess.run(
                ["git", "config", "user.name", "Test"],
                cwd=repository,
                check=True,
            )
            (repository / "AGENTS.md").write_text(
                "# Guidance\n\nRust application services own WyrmGrid business rules.\n",
                encoding="utf-8",
            )
            subprocess.run(["git", "add", "."], cwd=repository, check=True)
            subprocess.run(
                ["git", "commit", "-qm", "base"], cwd=repository, check=True
            )
            revision = subprocess.check_output(
                ["git", "rev-parse", "HEAD"], cwd=repository, text=True
            ).strip()
            artifacts = root / "artifacts"
            artifacts.mkdir()
            (artifacts / "parameters.json").write_text(
                json.dumps(
                    {
                        "mode": "ASK",
                        "source_revision": revision,
                        "allowed_paths": [],
                        "max_changed_files": 0,
                        "max_changed_lines": 0,
                    }
                ),
                encoding="utf-8",
            )
            (artifacts / "injected-guidance-evidence.json").write_text(
                json.dumps(
                    agent.record_injected_guidance_evidence(repository, revision)
                ),
                encoding="utf-8",
            )
            event_log = root / "events.jsonl"
            event_log.write_text(
                "\n".join(
                    json.dumps(event)
                    for event in (
                        {
                            "type": "tool_use",
                            "part": {
                                "tool": "read",
                                "state": {
                                    "status": "completed",
                                    "input": {
                                        "filePath": str(repository / "AGENTS.md")
                                    },
                                    "metadata": {
                                        "lineStart": 1,
                                        "lineEnd": 3,
                                    },
                                },
                            },
                        },
                        {
                            "type": "text",
                            "part": {
                                "text": (
                                    "Rust application services own WyrmGrid business rules.\n\n"
                                    "Citations:\n- `AGENTS.md:3-3`"
                                )
                            },
                        },
                    )
                )
                + "\n",
                encoding="utf-8",
            )
            args = argparse.Namespace(
                artifact_dir=str(artifacts),
                agent_worktree=str(repository),
                event_log=str(event_log),
                policy=str(POLICY_PATH),
            )
            agent.collect_agent_output(args)
            report = json.loads(
                (artifacts / "agent-output.json").read_text(encoding="utf-8")
            )
            self.assertEqual([], report["changed_paths"])
            self.assertEqual(
                [{"path": "AGENTS.md", "line_start": 3, "line_end": 3}],
                report["citations"],
            )
            self.assertFalse(event_log.exists())

    def test_citation_must_be_covered_by_read_and_ground_the_answer(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            repository = pathlib.Path(temporary)
            (repository / "AGENTS.md").write_text(
                "# Guidance\n\n"
                "Rust application services own WyrmGrid business rules.\n"
                "Svelte components present WyrmGrid state without mutating it.\n",
                encoding="utf-8",
            )
            citation = [{"path": "AGENTS.md", "line_start": 3, "line_end": 3}]
            agent.validate_citation_evidence(
                repository,
                "Rust application services own WyrmGrid business rules.",
                citation,
                {"AGENTS.md": [(1, 4)]},
            )
            with self.assertRaises(agent.PolicyError):
                agent.validate_citation_evidence(
                    repository,
                    "Rust application services own WyrmGrid business rules.",
                    citation,
                    {},
                )
            with self.assertRaises(agent.PolicyError):
                agent.validate_citation_evidence(
                    repository,
                    "Svelte components present WyrmGrid state without mutating it.",
                    citation,
                    {"AGENTS.md": [(1, 4)]},
                )
            with self.assertRaises(agent.PolicyError):
                agent.validate_citation_evidence(
                    repository,
                    (
                        "UI business rules must live in Svelte components, and "
                        "Tauri commands delegate business logic to the frontend."
                    ),
                    [{"path": "AGENTS.md", "line_start": 1, "line_end": 1}],
                    {"AGENTS.md": [(1, 4)]},
                )

    def test_root_guidance_requires_matching_immutable_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            repository = root / "repository"
            artifacts = root / "artifacts"
            repository.mkdir()
            artifacts.mkdir()
            (repository / "AGENTS.md").write_text(
                "# Guidance\n\nRust application services own WyrmGrid business rules.\n",
                encoding="utf-8",
            )
            revision = "a" * 40
            (artifacts / "injected-guidance-evidence.json").write_text(
                json.dumps(
                    agent.record_injected_guidance_evidence(repository, revision)
                ),
                encoding="utf-8",
            )
            injected_ranges = agent.validate_injected_guidance_evidence(
                repository, artifacts, revision
            )
            agent.validate_citation_evidence(
                repository,
                "Rust application services own WyrmGrid business rules.",
                [{"path": "AGENTS.md", "line_start": 3, "line_end": 3}],
                {},
                injected_ranges,
            )
            (repository / "AGENTS.md").write_text(
                "# Guidance\n\nSvelte components present WyrmGrid state without mutating it.\n",
                encoding="utf-8",
            )
            with self.assertRaises(agent.PolicyError):
                agent.validate_injected_guidance_evidence(
                    repository, artifacts, revision
                )

    def test_injected_guidance_does_not_cover_unread_documentation(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            repository = pathlib.Path(temporary)
            (repository / "AGENTS.md").write_text(
                "# Guidance\n\nRust application services own WyrmGrid business rules.\n",
                encoding="utf-8",
            )
            (repository / "architecture.md").write_text(
                "Rust application services own WyrmGrid business rules.\n",
                encoding="utf-8",
            )
            with self.assertRaises(agent.PolicyError):
                agent.validate_citation_evidence(
                    repository,
                    "Rust application services own WyrmGrid business rules.",
                    [
                        {
                            "path": "architecture.md",
                            "line_start": 1,
                            "line_end": 1,
                        }
                    ],
                    {},
                    {"AGENTS.md": [(1, 3)]},
                )

    def test_capture_rejects_missing_exact_citations(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            event_log = pathlib.Path(temporary) / "events.jsonl"
            event_log.write_text(
                json.dumps({"type": "text", "part": {"text": "No sources."}})
                + "\n",
                encoding="utf-8",
            )
            text = agent.extract_final_agent_text(event_log, 1024, 1024)
            with self.assertRaises(agent.PolicyError):
                agent.citations_from_text(text)

    def test_document_inventory_records_headings_hashes_and_lines(self) -> None:
        parameters = {
            "read_scope": ["AGENTS.md", "docs/architecture/decisions"]
        }
        entries = agent.build_inventory(ROOT, parameters, 4 * 1024 * 1024)
        by_path = {entry["path"]: entry for entry in entries}
        self.assertIn("AGENTS.md", by_path)
        self.assertEqual(64, len(by_path["AGENTS.md"]["sha256"]))
        self.assertTrue(by_path["AGENTS.md"]["headings"])
        self.assertEqual(
            "repository_guidance", by_path["AGENTS.md"]["document_kind"]
        )
        self.assertIn("line_end", by_path["AGENTS.md"]["headings"][0])

    def test_context_limit_keeps_large_files_inventory_only(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            (root / "AGENTS.md").write_text("# Rules\n", encoding="utf-8")
            (root / "small.md").write_text("# Small\nUseful.\n", encoding="utf-8")
            (root / "large.md").write_text(
                "# Large\n" + ("bounded context evidence\n" * 2000),
                encoding="utf-8",
            )
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(
                [
                    "git",
                    "-c",
                    "user.name=Test",
                    "-c",
                    "user.email=test@example.invalid",
                    "commit",
                    "-qm",
                    "base",
                ],
                cwd=root,
                check=True,
            )
            parameters = {
                "mode": "ASK",
                "read_scope": ["AGENTS.md", "small.md", "large.md"],
                "allowed_paths": [],
            }
            visible, excluded, _ = agent.select_model_visible_files(
                root,
                parameters["read_scope"],
                parameters,
                self.policy,
            )
            self.assertEqual(["AGENTS.md", "small.md"], visible)
            self.assertEqual("large.md", excluded[0]["path"])
            self.assertIn(excluded[0]["reason"], {"file_bytes", "file_lines"})

    def test_context_limit_rejects_an_exact_large_change_target(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            (root / "large.md").write_text(
                "# Large\n" + ("bounded context evidence\n" * 2000),
                encoding="utf-8",
            )
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            parameters = {
                "mode": "PATCH",
                "read_scope": [],
                "allowed_paths": ["large.md"],
            }
            with self.assertRaises(agent.PolicyError):
                agent.select_model_visible_files(
                    root,
                    ["large.md"],
                    parameters,
                    self.policy,
                )

    def test_diff_validator_rejects_out_of_scope_change_and_secret(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            subprocess.run(
                ["git", "config", "user.email", "test@example.invalid"],
                cwd=root,
                check=True,
            )
            subprocess.run(
                ["git", "config", "user.name", "Test"], cwd=root, check=True
            )
            (root / "allowed").mkdir()
            (root / "allowed" / "file.txt").write_text("before\n", encoding="utf-8")
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(["git", "commit", "-qm", "base"], cwd=root, check=True)
            parameters = {
                "mode": "PATCH",
                "allowed_paths": ["allowed"],
                "max_changed_files": 8,
                "max_changed_lines": 500,
            }
            (root / "outside.txt").write_text("changed\n", encoding="utf-8")
            with self.assertRaises(agent.PolicyError):
                agent.inspect_changes(root, parameters, self.policy, "HEAD")
            (root / "outside.txt").unlink()
            (root / "allowed" / "file.txt").write_text(
                "sk-proj-abcdefghijklmnop\n", encoding="utf-8"
            )
            with self.assertRaises(agent.PolicyError):
                agent.inspect_changes(root, parameters, self.policy, "HEAD")

    def test_rename_numstat_is_attributed_to_the_destination(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            subprocess.run(
                ["git", "config", "user.email", "test@example.invalid"],
                cwd=root,
                check=True,
            )
            subprocess.run(
                ["git", "config", "user.name", "Test"], cwd=root, check=True
            )
            (root / "allowed").mkdir()
            (root / "allowed" / "before.txt").write_text(
                "one\ntwo\n", encoding="utf-8"
            )
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(["git", "commit", "-qm", "base"], cwd=root, check=True)
            (root / "allowed" / "before.txt").rename(
                root / "allowed" / "after.txt"
            )
            subprocess.run(
                ["git", "add", "--intent-to-add", "allowed/after.txt"],
                cwd=root,
                check=True,
            )
            stats = agent.diff_numstat(root, "HEAD")
            self.assertIn("allowed/after.txt", stats)

    def test_agent_worktree_contains_only_declared_read_and_write_files(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary) / "repository"
            root.mkdir()
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            subprocess.run(
                ["git", "config", "user.email", "test@example.invalid"],
                cwd=root,
                check=True,
            )
            subprocess.run(
                ["git", "config", "user.name", "Test"], cwd=root, check=True
            )
            (root / "AGENTS.md").write_text("# Rules\n", encoding="utf-8")
            (root / "docs").mkdir()
            (root / "docs" / "selected.md").write_text(
                "# Selected\n", encoding="utf-8"
            )
            (root / "private").mkdir()
            (root / "private" / "hidden.md").write_text(
                "# Hidden\n", encoding="utf-8"
            )
            (root / "src").mkdir()
            (root / "src" / "allowed.py").write_text(
                "VALUE = 1\n", encoding="utf-8"
            )
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(["git", "commit", "-qm", "base"], cwd=root, check=True)
            revision = subprocess.check_output(
                ["git", "rev-parse", "HEAD"], cwd=root, text=True
            ).strip()
            artifacts = root / "ci-artifacts" / "ai-agent"
            artifacts.mkdir(parents=True)
            (artifacts / "parameters.json").write_text(
                json.dumps(
                    {
                        "mode": "FEATURE",
                        "source_revision": revision,
                        "request": "Add a scoped feature.",
                        "read_scope": ["docs/selected.md"],
                        "allowed_paths": ["src/allowed.py"],
                        "local_model_profile": "SCOPED_BUILDER_LOCAL",
                        "local_model": "qwen3-coder:30b",
                    }
                ),
                encoding="utf-8",
            )
            worktree = pathlib.Path(temporary) / "agent-worktree"
            args = argparse.Namespace(
                repository=str(root),
                policy=str(POLICY_PATH),
                artifact_dir=str(artifacts),
                agent_worktree=str(worktree),
            )
            try:
                agent.prepare_workspace(args)
                self.assertTrue((worktree / "AGENTS.md").is_file())
                self.assertTrue((worktree / "docs" / "selected.md").is_file())
                self.assertTrue((worktree / "src" / "allowed.py").is_file())
                self.assertFalse((worktree / "private" / "hidden.md").exists())
                guidance = json.loads(
                    (artifacts / "injected-guidance-evidence.json").read_text(
                        encoding="utf-8"
                    )
                )
                self.assertEqual(revision, guidance["source_revision"])
                self.assertEqual("AGENTS.md", guidance["files"][0]["path"])
                self.assertEqual(64, len(guidance["files"][0]["sha256"]))
                system_prompt = (artifacts / "agent-system.md").read_text(
                    encoding="utf-8"
                )
                self.assertIn("1 | # Rules", system_prompt)
            finally:
                subprocess.run(
                    ["git", "worktree", "remove", "--force", str(worktree)],
                    cwd=root,
                    check=False,
                )

    def test_validate_output_requires_real_commit_bound_citations(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            artifacts = pathlib.Path(temporary)
            revision = subprocess.check_output(
                ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True
            ).strip()
            (artifacts / "parameters.json").write_text(
                json.dumps({"mode": "ASK", "source_revision": revision}),
                encoding="utf-8",
            )
            (artifacts / "visible-files.json").write_text(
                json.dumps(["AGENTS.md"]),
                encoding="utf-8",
            )
            (artifacts / "agent-output.md").write_text("Result\n", encoding="utf-8")
            (artifacts / "agent-output.json").write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "mode": "ASK",
                        "source_revision": revision,
                        "summary": "Result",
                        "citations": [
                            {"path": "AGENTS.md", "line_start": 1, "line_end": 1}
                        ],
                    }
                ),
                encoding="utf-8",
            )
            args = argparse.Namespace(repository=str(ROOT), artifact_dir=str(artifacts))
            agent.validate_output(args)

    def test_registered_tests_apply_repair_to_current_draft_head(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary) / "repository"
            root.mkdir()
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            subprocess.run(
                ["git", "config", "user.email", "test@example.invalid"],
                cwd=root,
                check=True,
            )
            subprocess.run(
                ["git", "config", "user.name", "Test"], cwd=root, check=True
            )
            (root / "feature.txt").write_text("base\n", encoding="utf-8")
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(["git", "commit", "-qm", "base"], cwd=root, check=True)
            source_revision = subprocess.check_output(
                ["git", "rev-parse", "HEAD"], cwd=root, text=True
            ).strip()
            (root / "feature.txt").write_text(
                "base\ndraft\n", encoding="utf-8"
            )
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(["git", "commit", "-qm", "draft"], cwd=root, check=True)
            draft_revision = subprocess.check_output(
                ["git", "rev-parse", "HEAD"], cwd=root, text=True
            ).strip()
            (root / "feature.txt").write_text(
                "base\ndraft\nrepair\n", encoding="utf-8"
            )

            artifacts = pathlib.Path(temporary) / "artifacts"
            artifacts.mkdir()
            policy = json.loads(POLICY_PATH.read_text(encoding="utf-8"))
            policy["test_profiles"] = {
                "VERIFY_REPAIR": {
                    "description": "Check the complete draft and repair.",
                    "timeout_seconds": 30,
                    "commands": [
                        [
                            "python",
                            "-c",
                            (
                                "from pathlib import Path; "
                                "assert Path('feature.txt').read_text() == "
                                "'base\\ndraft\\nrepair\\n'"
                            ),
                        ]
                    ],
                }
            }
            policy_path = pathlib.Path(temporary) / "policy.json"
            policy_path.write_text(json.dumps(policy), encoding="utf-8")
            (artifacts / "parameters.json").write_text(
                json.dumps(
                    {
                        "source_revision": source_revision,
                        "test_profile": "VERIFY_REPAIR",
                    }
                ),
                encoding="utf-8",
            )
            test_worktree = pathlib.Path(temporary) / "test-worktree"
            args = argparse.Namespace(
                repository=str(root),
                policy=str(policy_path),
                artifact_dir=str(artifacts),
                test_worktree=str(test_worktree),
            )

            agent.run_tests(args)

            summary = json.loads(
                (artifacts / "tests" / "summary.json").read_text(encoding="utf-8")
            )
            self.assertEqual("passed", summary["outcome"])
            self.assertEqual(draft_revision, summary["base_revision"])


if __name__ == "__main__":
    unittest.main()
