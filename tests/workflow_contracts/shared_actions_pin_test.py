"""Every `leynos/shared-actions` reference pins one full commit.

The reading lives in ``shared_actions_pin``. The repository cases hold this
tree to the rule, and the constructed cases prove the reader over the shapes
this tree does not happen to use.
"""

from __future__ import annotations

import re
import typing as typ

import pytest
from shared_actions_pin import shared_actions_refs
from workflow_loader import load_workflow, repository_workflows

#: A full commit SHA. A tag or branch name is mutable, and a short SHA is
#: ambiguous, so neither is a pin.
FULL_SHA: typ.Final = re.compile(r"[0-9a-f]{40}")


def test_the_workflows_use_shared_actions() -> None:
    """The presence half: the rule below is not satisfied by an empty set.

    With no reference found, "every reference names one commit" holds
    vacuously, which is also what a reader that stopped matching would report.
    """
    assert shared_actions_refs(repository_workflows()), (
        "no leynos/shared-actions reference was found; this repository is "
        "expected to call setup-rust, generate-coverage and the uploader"
    )


def test_every_shared_actions_reference_pins_one_commit() -> None:
    """Scenario: one action is repinned and its siblings are not.

    Invariant: every reference names the same ref. A Dependabot group bump
    moves them together, so the rule costs nothing on the routine path and
    catches a hand edit or a new call pinned elsewhere.
    """
    refs = shared_actions_refs(repository_workflows())
    assert len(refs) == 1, (
        "every leynos/shared-actions reference must pin the same commit; "
        f"found {len(refs)}: "
        + "; ".join(f"{ref or '(no ref)'} at {', '.join(where)}" for ref, where in refs.items())
    )


def test_the_shared_actions_pin_is_a_full_commit_sha() -> None:
    """Scenario: the one shared ref is a tag or a branch.

    Invariant: the ref is forty hexadecimal characters. One mutable ref shared
    by every call would satisfy the single-commit rule while pinning nothing.
    """
    bad = [ref for ref in shared_actions_refs(repository_workflows()) if not FULL_SHA.fullmatch(ref)]
    assert not bad, f"leynos/shared-actions must be pinned to a full commit SHA; got {bad}"


def test_the_reader_sees_every_shape_of_reference() -> None:
    """Steps, repository-root actions and job-level workflows are read, in any case.

    A mixed-case owner is the same action to GitHub; matched exactly it would
    escape both rules above while pinning `main`.
    """
    document = load_workflow(
        "on: pull_request\n"
        "jobs:\n"
        "  build:\n"
        "    steps:\n"
        "      - uses: leynos/shared-actions/.github/actions/setup-rust@aaaa\n"
        "      - uses: Leynos/Shared-Actions/.github/actions/generate-coverage@main\n"
        "      - uses: actions/checkout@v5\n"
        "      - uses: leynos/shared-actions@cccc\n"
        "  merge:\n"
        "    uses: leynos/shared-actions/.github/workflows/automerge.yml@bbbb\n"
    )

    assert sorted(shared_actions_refs({"w.yml": document})) == [
        "aaaa",
        "bbbb",
        "cccc",
        "main",
    ]


@pytest.mark.parametrize(
    "uses",
    [
        pytest.param("actions/checkout@v5", id="other-owner"),
        pytest.param("leynos/shared-actions-fork/.github/actions/x@main", id="similar-name"),
        pytest.param("./.github/actions/local", id="local"),
    ],
)
def test_the_reader_ignores_other_repositories(uses: str) -> None:
    """The narrowness half: only this one repository is read."""
    document = load_workflow(f"on: push\njobs:\n  j:\n    steps:\n      - uses: {uses}\n")

    assert shared_actions_refs({"w.yml": document}) == {}
