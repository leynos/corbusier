"""Read every `leynos/shared-actions` reference in the workflows.

The actions in that repository are developed together, and the interfaces
between them move together: `setup-rust` exports what `generate-coverage`
reads, and the uploader reads what the generator writes. A tree pinning them
at different commits runs a combination nobody has run. So every reference
must name one commit, and ``shared_actions_pin_test`` holds the tree to that.

The owner and repository name are matched case-insensitively, because GitHub
resolves them that way: `Leynos/Shared-Actions/...@main` is the same action,
and an exact match would not see it at all.
"""

from __future__ import annotations

import typing as typ

from codescene_placement_reader import calls

if typ.TYPE_CHECKING:
    import collections.abc as cabc

    from workflow_loader import Document

#: The repository a `uses:` value names, lowercased for comparison. A reference
#: is this name followed by `/` (an action or workflow path) or `@` (an action
#: at the repository root); requiring the boundary keeps a similarly named
#: repository such as `leynos/shared-actions-fork` out.
SHARED_ACTIONS: typ.Final[str] = "leynos/shared-actions"


def shared_actions_refs(documents: cabc.Mapping[str, Document]) -> dict[str, list[str]]:
    """Return each ref a shared-actions reference pins, with where it appears.

    Parameters
    ----------
    documents : Mapping[str, Document]
        Every workflow in the tree, keyed by file name.

    Returns
    -------
    dict[str, list[str]]
        Each distinct ref (the text after `@`, empty when there is none)
        mapped to the `file: uses` locations that pin it, in file order.

    Examples
    --------
    >>> shared_actions_refs({"ci.yml": {"jobs": {"b": {"steps": [
    ...     {"uses": "Leynos/Shared-Actions/.github/actions/setup-rust@abc"}]}}}})
    {'abc': ['ci.yml: Leynos/Shared-Actions/.github/actions/setup-rust@abc']}
    """
    refs: dict[str, list[str]] = {}
    for name, document in documents.items():
        for call in calls(document):
            uses = str(call.get("uses", "")).strip()
            if not uses.lower().startswith((f"{SHARED_ACTIONS}/", f"{SHARED_ACTIONS}@")):
                continue
            _, _, ref = uses.partition("@")
            refs.setdefault(ref, []).append(f"{name}: {uses}")
    return refs
