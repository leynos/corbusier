"""Reading machinery for the workflow contracts in this directory.

This module gets facts out of a workflow tree and holds no opinion about
them; the assertions live in the contracts that call it, and the readings are
proved against constructed trees in ``workflow_reading_test``. Every reader
takes its documents as an argument, so a fixture tree exercises exactly the
code the contracts run.

Loading is ``workflow_loader``'s, which refuses a duplicated mapping key.
``triggers`` reads ``on:`` as a scalar, a sequence or a mapping, under both
the string key and YAML 1.1's boolean ``True``, and refuses anything else. A
mapping-only reader stringifies ``on: [push, pull_request]`` into one key
that matches no trigger, and a reader that returns nothing for an unknown
shape lets the workflow escape every pull-request clause.

The CV-005 CodeScene placement contract, which also followed local calls to
find what a pull request reaches, now runs from the shared library through
``make test-workflow-contracts``.
"""

from __future__ import annotations

import typing as typ

if typ.TYPE_CHECKING:
    from workflow_loader import Document

def triggers(document: Document) -> dict[object, object]:
    """Return a workflow's triggers as a mapping of name to configuration.

    Parameters
    ----------
    document
        One parsed workflow document.

    Returns
    -------
    dict[object, object]
        Trigger name to its configuration; a scalar or sequence form maps
        each name to ``None``.

    Raises
    ------
    ValueError
        When ``on:`` is missing, is declared under both the string key and
        YAML 1.1's boolean ``True``, or is not a scalar, sequence or mapping of
        names, since a workflow whose triggers cannot be read cannot be
        classified as outside the pull-request lane either.

    Examples
    --------
    >>> triggers({True: ["push", "pull_request"]})
    {'push': None, 'pull_request': None}
    """
    if "on" in document and True in document:
        # GitHub merges the two, so reading either alone is blind to the
        # other's triggers.
        message = "a workflow declaring both `on:` and `'on':` is refused"
        raise ValueError(message)
    for key in ("on", True):
        if key not in document:
            continue
        match document[key]:
            case dict() as mapping:
                return mapping
            case str() as event:
                return {event: None}
            case list() as events if all(
                isinstance(event, str) for event in events
            ):
                return dict.fromkeys(events)
            case other:
                message = f"unsupported `on:` shape {other!r}"
                raise ValueError(message)
    message = "a workflow with no `on:` block cannot be classified"
    raise ValueError(message)


def jobs(document: Document) -> dict[object, Document]:
    """Return a workflow's jobs, skipping any that are not mappings.

    Parameters
    ----------
    document
        One parsed workflow document.

    Returns
    -------
    dict[object, Document]
        Job id to job definition.
    """
    raw = document.get("jobs")
    if not isinstance(raw, dict):
        return {}
    return {name: job for name, job in raw.items() if isinstance(job, dict)}


def steps(document: Document) -> list[Document]:
    """Return every mapping step in every job, in document order.

    Parameters
    ----------
    document
        One parsed workflow document.

    Returns
    -------
    list[Document]
        Every step that is a mapping, across all jobs.
    """
    return [
        step
        for job in jobs(document).values()
        for step in (job.get("steps") or [])
        if isinstance(step, dict)
    ]


def calls(document: Document) -> list[Document]:
    """Return every step, plus every job that is itself a workflow call.

    A job calling a reusable workflow carries ``uses:`` on the job and has no
    steps, so a reader of steps alone misses the one shape that can run
    another repository's workflow.

    Parameters
    ----------
    document
        One parsed workflow document.

    Returns
    -------
    list[Document]
        The calling jobs first, then every step.
    """
    return [job for job in jobs(document).values() if "uses" in job] + steps(
        document
    )
