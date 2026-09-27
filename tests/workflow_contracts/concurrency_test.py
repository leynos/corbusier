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

The rules live in ``concurrency_rules`` and are proved against constructed
workflows in ``concurrency_rules_test``; this module holds the repository's
own workflows to them. Run via ``make test-workflow-contracts``.
"""

from __future__ import annotations

import typing as typ

import pytest
from codescene_placement_reader import triggers
from concurrency_rules import (
    CANCEL_EXPRESSION,
    GROUP_EXPRESSION,
    cancellation_is_conditioned,
    declares_group,
    group_is_ruled,
    pull_request_workflows,
)
from workflow_loader import repository_workflows

#: Workflows known to start on `pull_request`. Discovery below is dynamic so a
#: new workflow is covered the day it lands, but a dynamic list that silently
#: empties turns every parametrized test into a vacuous pass. This names the
#: floor discovery must still reach.
KNOWN_PULL_REQUEST_WORKFLOWS: typ.Final = frozenset({"ci.yml"})

PULL_REQUEST_WORKFLOWS: typ.Final = pull_request_workflows(repository_workflows())


def test_discovery_still_finds_the_known_pull_request_workflows() -> None:
    """Discovery reaches its floor, so the parametrized contracts are not empty.

    Every test below is parametrized over a list built by reading the
    workflows. If that read were to break, the list would empty and each
    contract would report as passed having asserted nothing.
    """
    missing = sorted(KNOWN_PULL_REQUEST_WORKFLOWS - set(PULL_REQUEST_WORKFLOWS))
    assert not missing, (
        f"these workflows start on pull_request but discovery missed them: "
        f"{', '.join(missing)}; the contracts below would pass without "
        "asserting anything about them"
    )


@pytest.mark.parametrize("name", sorted(repository_workflows()))
def test_every_workflow_declares_a_trigger_set_the_reader_models(name: str) -> None:
    """No workflow's `on:` defeats the reader that decides what is in scope.

    The reader refuses a shape it cannot model rather than returning nothing,
    so a workflow it cannot read fails here by name instead of dropping out of
    discovery and taking every contract below with it.
    """
    try:
        triggers(repository_workflows()[name])
    except ValueError as error:
        pytest.fail(f"{name}: {error}")


@pytest.mark.parametrize("name", PULL_REQUEST_WORKFLOWS)
def test_every_pull_request_workflow_declares_a_concurrency_group(name: str) -> None:
    """A workflow a pull request starts declares a concurrency group.

    Without one, every push to the branch leaves its predecessor running to
    completion on a paid runner.
    """
    assert declares_group(repository_workflows()[name]), (
        f"{name} starts on pull_request and must declare concurrency.group; "
        "without it a superseded run holds a runner until it finishes"
    )


@pytest.mark.parametrize("name", PULL_REQUEST_WORKFLOWS)
def test_the_group_is_keyed_on_the_pull_request_with_a_run_fallback(
    name: str,
) -> None:
    """The group is the pull request, or the run itself where there is none.

    Successive pushes to one pull request share a group, so the newer run
    cancels the older; two pull requests never share one; and a dispatch
    falls back to its own run id, so no later dispatch can replace it while
    it is pending.
    """
    assert group_is_ruled(repository_workflows()[name]), (
        f"{name} must set concurrency.group to {GROUP_EXPRESSION!r}"
    )


@pytest.mark.parametrize("name", PULL_REQUEST_WORKFLOWS)
def test_cancellation_is_conditioned_on_the_event(name: str) -> None:
    """Cancellation applies to pull requests only, not to dispatches.

    A literal `true` reads as a stricter setting and is a regression: it
    would cancel runs that have no successor waiting.
    """
    assert cancellation_is_conditioned(repository_workflows()[name]), (
        f"{name} must set cancel-in-progress to {CANCEL_EXPRESSION!r}; a "
        "missing value leaves superseded runs in flight and a literal true "
        "also cancels runs with no successor"
    )
