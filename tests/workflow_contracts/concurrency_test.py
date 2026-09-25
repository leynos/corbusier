"""Contracts cancelling superseded pull-request runs.

Every push to a pull request starts a fresh run of each gate, and the run
already in flight is answering a question about a commit nobody will merge.
Left alone it holds a runner until it finishes, so the branch pays twice for
one answer. A concurrency group keyed on the pull request makes the newer run
cancel the older one.

Cancellation has to stay conditioned on the event. A literal
``cancel-in-progress: true`` would also cancel a push to `main`, a schedule,
and a dispatch, none of which has a successor waiting: the run that writes the
warm cache on `main` would be killed by the next merge, and the coverage
history would gain holes. The condition is therefore part of the contract, not
an implementation detail, and `test_cancellation_is_conditioned_on_the_event`
fails on the literal.

The group also has to distinguish one pull request from another, so it is
keyed on the pull-request number. A constant group would let one branch
cancel another's gates, and a group built from ``github.run_id`` alone is
unique per run and so cancels nothing. Outside a pull request the number is
empty, and the group falls back to ``github.run_id``: a shared fallback such
as ``github.ref`` would let a third dispatch replace a pending second one
that was meant to complete. The run id is allowed in that fallback position
and nowhere else, so the group is compared whole.

Only `pull_request` is in scope. A `pull_request_target` workflow runs against
the base repository to carry a token, and the workflows that use it here automate
pull-request housekeeping rather than build; cancelling an auto-merge
mid-flight is a hazard with no minutes to win.

Run via ``make test-workflow-contracts``.
"""

from __future__ import annotations

import typing as typ

import pytest
from workflow_loader import WORKFLOW_DIR, WORKFLOW_SUFFIXES, load_workflow

if typ.TYPE_CHECKING:
    from pathlib import Path

#: The exact `cancel-in-progress` expression every pull-request workflow
#: carries. Comparing against one string rather than searching for a substring
#: is what makes the literal `true` mutation fail: `true` is a YAML boolean and
#: never equals this.
CANCEL_EXPRESSION = "${{ github.event_name == 'pull_request' }}"

#: The trigger that puts a workflow in scope. `pull_request_target` is
#: deliberately absent; see the module docstring.
PULL_REQUEST = "pull_request"

#: The exact group every pull-request workflow carries: keyed on the pull
#: request, falling back to the run id where there is none. Compared whole,
#: because each plausible variant fails differently: `github.run_id` alone
#: cancels nothing, a constant cancels other branches, and a `github.ref`
#: fallback lets a third dispatch replace a pending second one.
GROUP_EXPRESSION = "${{ github.workflow }}-${{ github.event.pull_request.number || github.run_id }}"

#: Workflows known to start on `pull_request`. Discovery below is dynamic so a
#: new workflow is covered the day it lands, but a dynamic list that silently
#: empties turns every parametrized test into a vacuous pass. This names the
#: floor discovery must still reach.
KNOWN_PULL_REQUEST_WORKFLOWS: frozenset[str] = frozenset(
    {
        "ci.yml",
    }
)


def _load(path: Path) -> dict[str, object]:
    """Read and parse one workflow file.

    Parameters
    ----------
    path
        Workflow file to read.

    Returns
    -------
    dict
        The parsed workflow document.

    Raises
    ------
    AssertionError
        If the document does not parse as a non-empty mapping, which means it
        is not a workflow at all.
    DuplicateKeyError
        If any mapping declares a key twice; the strict loader refuses the
        document rather than keeping the last value.
    """
    document = load_workflow(path.read_text(encoding="utf-8"))
    if not document:
        message = f"{path.name} must parse as a mapping"
        raise AssertionError(message)
    return document


def _workflow_paths() -> list[Path]:
    """Return every workflow file in the repository.

    Returns
    -------
    list of Path
        Workflow paths sorted by name, so parametrized tests report in a
        stable order.
    """
    return sorted(
        path
        for path in WORKFLOW_DIR.iterdir()
        if path.is_file() and path.suffix.lower() in WORKFLOW_SUFFIXES
    )


def _trigger_names(document: dict[str, object]) -> frozenset[str] | None:
    """Return the event names a workflow declares under `on:`.

    GitHub accepts three shapes: a mapping of event to configuration, a list
    of event names, and a bare event name. All three are read here. A reader
    that modelled only the mapping would drop a workflow written either other
    way out of discovery, and a contract over a filtered list reports nothing
    at all about a workflow it never sees.

    PyYAML also resolves an unquoted `on:` key to the boolean ``True``, so
    both spellings of the key are read.

    Parameters
    ----------
    document
        A parsed workflow document.

    Returns
    -------
    frozenset of str, or None
        The declared event names, empty when the workflow declares no `on:`
        at all, in which case nothing can start it. ``None`` when `on:` is
        present in a shape this reader does not model; the caller decides
        what to do about that, and
        `test_every_workflow_declares_a_trigger_set_this_reader_models`
        reports it by name rather than letting discovery drop it in silence.
    """
    declared = document.get("on", document.get(True))
    if declared is None:
        return frozenset()
    if isinstance(declared, dict):
        return frozenset(key for key in declared if isinstance(key, str))
    if isinstance(declared, list):
        return frozenset(event for event in declared if isinstance(event, str))
    if isinstance(declared, str):
        return frozenset({declared})
    return None


def _pull_request_workflows() -> list[Path]:
    """Return every workflow a pull request can start.

    Returns
    -------
    list of Path
        Workflow paths declaring a `pull_request` trigger, in name order.
    """
    return [
        path
        for path in _workflow_paths()
        if PULL_REQUEST in (_trigger_names(_load(path)) or frozenset())
    ]


def _concurrency(path: Path) -> dict[str, object]:
    """Return a workflow's top-level concurrency mapping.

    Parameters
    ----------
    path
        Workflow file to read.

    Returns
    -------
    dict
        The concurrency mapping, empty when the workflow declares none or
        declares the shorthand string form, which cannot carry
        `cancel-in-progress` at all.
    """
    declared = _load(path).get("concurrency")
    return declared if isinstance(declared, dict) else {}


PULL_REQUEST_WORKFLOWS = _pull_request_workflows()
WORKFLOW_IDS = [path.name for path in PULL_REQUEST_WORKFLOWS]


def test_discovery_still_finds_the_known_pull_request_workflows() -> None:
    """Discovery reaches its floor, so the parametrized contracts are not empty.

    Every test below is parametrized over a list built by reading the workflow
    estate. If that read were to break, or the `on:` key were to change shape,
    the list would empty and each contract would report as passed having
    asserted nothing.
    """
    discovered = set(WORKFLOW_IDS)
    missing = sorted(KNOWN_PULL_REQUEST_WORKFLOWS - discovered)
    assert not missing, (
        f"these workflows start on pull_request but discovery missed them: "
        f"{', '.join(missing)}; the contracts below would pass without "
        "asserting anything about them"
    )


def test_every_workflow_declares_a_trigger_set_this_reader_models() -> None:
    """No workflow's `on:` defeats the reader that decides what is in scope.

    Discovery filters on the event names it can read, and a workflow whose
    `on:` the reader cannot model is dropped from that filter. Dropped
    silently it would take every contract below with it, each passing while
    saying nothing about that workflow. This is the half that makes the
    silence loud.
    """
    unreadable = sorted(
        path.name for path in _workflow_paths() if _trigger_names(_load(path)) is None
    )
    assert not unreadable, (
        f"these workflows declare an `on:` this reader does not model: "
        f"{', '.join(unreadable)}; each is dropped from discovery, so every "
        "contract below would pass without asserting anything about it"
    )


@pytest.mark.parametrize("workflow", PULL_REQUEST_WORKFLOWS, ids=WORKFLOW_IDS)
def test_every_pull_request_workflow_declares_a_concurrency_group(
    workflow: Path,
) -> None:
    """A workflow a pull request starts declares a concurrency group.

    Without one, every push to the branch leaves its predecessor running to
    completion on a paid runner.
    """
    group = _concurrency(workflow).get("group")
    assert isinstance(group, str) and group.strip(), (
        f"{workflow.name} starts on pull_request and must declare "
        "concurrency.group; without it a superseded run holds a runner until "
        "it finishes"
    )


@pytest.mark.parametrize("workflow", PULL_REQUEST_WORKFLOWS, ids=WORKFLOW_IDS)
def test_the_group_is_keyed_on_the_pull_request_with_a_run_fallback(
    workflow: Path,
) -> None:
    """The group is the pull request, or the run itself where there is none.

    Successive pushes to one pull request share a group, so the newer run
    cancels the older; two pull requests never share one, so neither cancels
    the other's gates; and a dispatch falls back to its own run id, so no
    later dispatch can replace it while it is pending. A run id anywhere but
    that fallback position would cancel nothing, which is why the group is
    compared whole rather than searched for its parts.
    """
    group = _concurrency(workflow).get("group")
    assert group == GROUP_EXPRESSION, (
        f"{workflow.name} must set concurrency.group to {GROUP_EXPRESSION!r}, "
        f"not {group!r}"
    )


@pytest.mark.parametrize("workflow", PULL_REQUEST_WORKFLOWS, ids=WORKFLOW_IDS)
def test_cancellation_is_conditioned_on_the_event(workflow: Path) -> None:
    """Cancellation applies to pull requests only, not to pushes or schedules.

    A literal `true` here reads as a stricter setting and is a regression: it
    would cancel the run on `main` that writes the warm cache and records
    coverage, which no later run repeats.
    """
    declared = _concurrency(workflow).get("cancel-in-progress")
    assert declared == CANCEL_EXPRESSION, (
        f"{workflow.name} must set cancel-in-progress to "
        f"{CANCEL_EXPRESSION!r}, not {declared!r}; a missing value leaves "
        "superseded runs in flight and a literal true also cancels pushes to "
        "main, schedules, and dispatches"
    )
