"""Rules for cancelling superseded pull-request runs, over parsed workflows.

``concurrency_test`` holds this repository's workflows to these rules, and
``concurrency_rules_test`` drives the same functions with constructed
workflows. Each function takes a parsed document rather than a path, so a
fixture exercises exactly the code the repository contract runs.

Triggers are read by ``codescene_placement_reader.triggers``, which reads the
scalar, sequence and mapping forms under both the ``on`` string key and the
boolean ``True`` YAML 1.1 makes of it. It refuses a workflow declaring both
keys, a missing ``on:`` and any shape it cannot model, so no workflow can
leave discovery in silence.
"""

from __future__ import annotations

import typing as typ

from codescene_placement_reader import triggers

if typ.TYPE_CHECKING:
    from collections.abc import Mapping

    from workflow_loader import Document

#: The trigger that puts a workflow in scope. `pull_request_target` is
#: deliberately absent: those workflows automate pull-request housekeeping
#: rather than build, and cancelling an auto-merge mid-flight is a hazard with
#: no minutes to win.
PULL_REQUEST: typ.Final = "pull_request"

#: The exact `cancel-in-progress` expression every pull-request workflow
#: carries. A literal `true` is a YAML boolean and never equals it.
CANCEL_EXPRESSION: typ.Final = "${{ github.event_name == 'pull_request' }}"

#: The exact group every pull-request workflow carries: keyed on the pull
#: request, falling back to the run id where there is none. Compared whole,
#: because each plausible variant fails differently: `github.run_id` alone
#: cancels nothing, a constant cancels other branches, and a `github.ref`
#: fallback lets a third dispatch replace a pending second one.
GROUP_EXPRESSION: typ.Final = (
    "${{ github.workflow }}-"
    "${{ github.event.pull_request.number || github.run_id }}"
)


def pull_request_workflows(documents: Mapping[str, Document]) -> list[str]:
    """Return the workflows a `pull_request` event starts, by file name.

    Raises
    ------
    ValueError
        When any workflow's triggers cannot be read, so an unreadable workflow
        fails discovery instead of dropping out of it.

    Examples
    --------
    >>> pull_request_workflows({"ci.yml": {True: ["pull_request"]},
    ...                         "main.yml": {True: "push"}})
    ['ci.yml']
    """
    return sorted(
        name
        for name, document in documents.items()
        if PULL_REQUEST in triggers(document)
    )


def concurrency(document: Document) -> dict[object, object]:
    """Return a workflow's top-level concurrency mapping.

    The shorthand string form reads as empty: it names a group but cannot
    carry `cancel-in-progress`, so every rule below fails on it.

    Examples
    --------
    >>> concurrency({"concurrency": "ci"})
    {}
    """
    declared = document.get("concurrency")
    return declared if isinstance(declared, dict) else {}


def declares_group(document: Document) -> bool:
    """Return whether a workflow declares a non-empty concurrency group.

    Examples
    --------
    >>> declares_group({"concurrency": {"group": "x"}})
    True
    """
    group = concurrency(document).get("group")
    return isinstance(group, str) and bool(group.strip())


def group_is_ruled(document: Document) -> bool:
    """Return whether the group is exactly the ruled expression.

    Examples
    --------
    >>> group_is_ruled({"concurrency": {"group": GROUP_EXPRESSION}})
    True
    """
    return concurrency(document).get("group") == GROUP_EXPRESSION


def cancellation_is_conditioned(document: Document) -> bool:
    """Return whether cancellation is exactly the pull-request condition.

    Examples
    --------
    >>> cancellation_is_conditioned(
    ...     {"concurrency": {"cancel-in-progress": True}})
    False
    """
    return concurrency(document).get("cancel-in-progress") == CANCEL_EXPRESSION
