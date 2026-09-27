"""Prove the cancellation rules against constructed workflows.

The repository's workflows use one trigger shape and one concurrency block,
so a rule that mishandles any other shape passes over them either way. Each
case here builds the shape it names and drives the same function
``concurrency_test`` runs.
"""

from __future__ import annotations

import textwrap
import typing as typ

import pytest
from concurrency_rules import (
    CANCEL_EXPRESSION,
    GROUP_EXPRESSION,
    cancellation_is_conditioned,
    declares_group,
    group_is_ruled,
    pull_request_workflows,
)
from workflow_loader import load_workflow, read_workflows

if typ.TYPE_CHECKING:
    from pathlib import Path

#: A compliant concurrency block, as the template the cases perturb.
GOOD_BLOCK: typ.Final = {
    "group": GROUP_EXPRESSION,
    "cancel-in-progress": CANCEL_EXPRESSION,
}


@pytest.mark.parametrize(
    "trigger",
    [
        pytest.param("on: pull_request\n", id="scalar"),
        pytest.param("'on': pull_request\n", id="scalar-quoted-key"),
        pytest.param("on: [push, pull_request]\n", id="sequence"),
        pytest.param("'on': [push, pull_request]\n", id="sequence-quoted-key"),
        pytest.param("on:\n  pull_request:\n    types: [opened]\n", id="mapping"),
        pytest.param("'on':\n  pull_request:\n", id="mapping-quoted-key"),
    ],
)
def test_every_trigger_form_is_discovered(trigger: str) -> None:
    """Scalar, sequence and mapping, under the boolean and the string key.

    Parsed through the strict loader, since only a resolving loader turns an
    unquoted `on` into the boolean `True`.
    """
    documents = {"w.yml": load_workflow(trigger + "jobs: {}\n")}

    assert pull_request_workflows(documents) == ["w.yml"]


@pytest.mark.parametrize(
    "trigger",
    [
        pytest.param("on: push\n", id="scalar"),
        pytest.param("on: [push, workflow_dispatch]\n", id="sequence"),
        pytest.param("on:\n  pull_request_target:\n", id="target"),
    ],
)
def test_a_workflow_without_pull_request_is_not_discovered(trigger: str) -> None:
    """The narrowness half: `pull_request_target` and pushes stay out."""
    documents = {"w.yml": load_workflow(trigger + "jobs: {}\n")}

    assert pull_request_workflows(documents) == []


@pytest.mark.parametrize(
    "text",
    [
        pytest.param("jobs: {}\n", id="missing"),
        pytest.param("on: 17\njobs: {}\n", id="number"),
        pytest.param("on: [push, {pull_request: null}]\njobs: {}\n", id="mixed-sequence"),
        pytest.param("on: push\n'on': pull_request\njobs: {}\n", id="both-keys"),
    ],
)
def test_an_unreadable_trigger_fails_discovery(text: str) -> None:
    """Discovery refuses rather than dropping the workflow.

    With both keys, a reader that picked one would see only `push` and miss
    the `pull_request` GitHub also runs.
    """
    with pytest.raises(ValueError, match="on:"):
        pull_request_workflows({"w.yml": load_workflow(text)})


def test_discovery_reads_a_directory_through_the_strict_loader(tmp_path: Path) -> None:
    """Discovery over files, as the repository contract runs it."""
    (tmp_path / "ci.yml").write_text("on: [pull_request]\njobs: {}\n", encoding="utf-8")
    (tmp_path / "main.YAML").write_text("on: push\njobs: {}\n", encoding="utf-8")

    assert pull_request_workflows(read_workflows(tmp_path)) == ["ci.yml"]


def test_the_compliant_block_passes_every_rule() -> None:
    """The template itself is compliant, so each failure below is its mutation."""
    document = {"concurrency": dict(GOOD_BLOCK)}

    assert declares_group(document)
    assert group_is_ruled(document)
    assert cancellation_is_conditioned(document)


@pytest.mark.parametrize(
    "block",
    [
        pytest.param(None, id="no-block"),
        pytest.param("ci", id="shorthand-string"),
        pytest.param({"cancel-in-progress": CANCEL_EXPRESSION}, id="no-group"),
        pytest.param({**GOOD_BLOCK, "group": "  "}, id="blank-group"),
    ],
)
def test_a_missing_group_is_refused(block: object) -> None:
    """No group, a blank one, or the shorthand form all fail the first rule."""
    document = {} if block is None else {"concurrency": block}

    assert not declares_group(document)


@pytest.mark.parametrize(
    "group",
    [
        pytest.param(GROUP_EXPRESSION.replace("github.run_id", "github.ref"), id="ref-fallback"),
        pytest.param("${{ github.workflow }}-${{ github.run_id }}", id="run-id-alone"),
        pytest.param(
            "${{ github.workflow }}-${{ github.run_id || github.event.pull_request.number }}",
            id="run-id-first",
        ),
        pytest.param("ci", id="constant"),
        pytest.param(GROUP_EXPRESSION.replace("github.run_id", "github.sha"), id="sha-fallback"),
    ],
)
def test_a_group_other_than_the_ruled_one_is_refused(group: str) -> None:
    """Every plausible variant of the group fails, each for its own reason."""
    document = {"concurrency": {**GOOD_BLOCK, "group": group}}

    assert declares_group(document)
    assert not group_is_ruled(document)


@pytest.mark.parametrize(
    "cancel",
    [
        pytest.param(True, id="literal-true"),
        pytest.param(False, id="literal-false"),
        pytest.param("${{ github.event_name != 'push' }}", id="other-condition"),
        pytest.param(None, id="missing"),
    ],
)
def test_unconditioned_cancellation_is_refused(cancel: object) -> None:
    """Only the pull-request condition passes; a literal `true` is the regression."""
    block = {**GOOD_BLOCK, "cancel-in-progress": cancel}
    if cancel is None:
        del block["cancel-in-progress"]

    assert not cancellation_is_conditioned({"concurrency": block})


def test_a_workflow_block_parsed_from_yaml_is_read() -> None:
    """The rules read the block as the loader yields it, not a hand-built dict."""
    document = load_workflow(
        textwrap.dedent(
            f"""\
            on: pull_request
            concurrency:
              group: {GROUP_EXPRESSION}
              cancel-in-progress: {CANCEL_EXPRESSION}
            jobs: {{}}
            """
        )
    )

    assert group_is_ruled(document)
    assert cancellation_is_conditioned(document)
