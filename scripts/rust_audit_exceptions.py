#!/usr/bin/env -S uv run python
# /// script
# requires-python = ">=3.13"
# dependencies = []
# ///
"""Read and validate the dated exceptions in ``.cargo/audit.toml``.

`cargo-audit` takes a bare list of advisory identifiers and rejects any key it
does not know, so an expiry date and a justification cannot be fields of that
file. They live in a fixed comment block above the list instead, and this
module is what makes the block load-bearing rather than decorative: `make
rust-audit` refuses to run while an ignored advisory is undated, malformed,
expired, or listed without a block.

That is the rule `frontend-pwa/security/audit-exceptions.json` already applies
to the frontend. An entry is a deferral, not a fix, and one that cannot expire
is not a deferral at all.
"""

from __future__ import annotations

import collections
import datetime as dt
import sys
import typing as typ
from pathlib import Path

from audit_exception_blocks import (
    AuditExceptionError,
    Exception_,
    ignored_advisories,
    parse_blocks,
)

#: The audit configuration, relative to the repository root.
AUDIT_CONFIG: typ.Final[Path] = Path(".cargo/audit.toml")

__all__ = [
    "AUDIT_CONFIG",
    "AuditExceptionError",
    "Exception_",
    "faults",
    "ignored_advisories",
    "main",
    "parse_blocks",
    "read_config",
]


def faults(text: str, today: dt.date) -> list[str]:
    """Return every reason the exceptions are not acceptable, in order.

    An advisory with more than one block is refused, since the blocks are
    competing statements about one ignore. Both directions are checked. An
    ignored advisory with no block is an
    undocumented exception, and a block with no ignored advisory is a stale
    justification for something the audit no longer suppresses; each reads as
    the other from the file alone, and only reporting both keeps the two
    lists one statement.

    Parameters
    ----------
    text : str
        The audit configuration's text.
    today : datetime.date
        The date to judge expiry against, taken by the caller so the rule can
        be driven over dates this repository is not on.

    Returns
    -------
    list[str]
        Human-readable faults; empty when the exceptions are acceptable.
    """
    blocks = parse_blocks(text)
    ignored = ignored_advisories(text)
    return [
        *_duplicate_faults(blocks),
        *_undocumented_faults(blocks, ignored),
        *_unused_faults(blocks, ignored),
        *_expired_faults(blocks, ignored, today),
    ]


def _duplicate_faults(blocks: list[Exception_]) -> list[str]:
    """Return a fault for each advisory that carries more than one block."""
    counts = collections.Counter(block.advisory for block in blocks)
    return [_duplicated(advisory, count) for advisory, count in counts.items() if count > 1]


def _undocumented_faults(blocks: list[Exception_], ignored: list[str]) -> list[str]:
    """Return a fault for each ignored advisory that has no block."""
    documented = {block.advisory for block in blocks}
    return [_undocumented(advisory) for advisory in ignored if advisory not in documented]


def _unused_faults(blocks: list[Exception_], ignored: list[str]) -> list[str]:
    """Return a fault for each block whose advisory is not ignored, once each."""
    advisories = dict.fromkeys(block.advisory for block in blocks)
    return [_unused(advisory) for advisory in advisories if advisory not in ignored]


def _expired_faults(
    blocks: list[Exception_], ignored: list[str], today: dt.date
) -> list[str]:
    """Return a fault for each block of an ignored advisory that has expired.

    Every block is judged, not one per advisory: a later current block must
    not hide an earlier expired one.
    """
    return [
        _expired(block)
        for block in blocks
        if block.advisory in ignored and block.expires_at < today
    ]


def _duplicated(advisory: str, count: int) -> str:
    """Return the fault for an advisory carrying more than one block.

    Two blocks are two statements about one ignore, and at most one of them
    can be the reason it stands; the file must say which.

    Examples
    --------
    >>> _duplicated("RUSTSEC-2026-0258", 2).startswith("RUSTSEC-2026-0258 has 2")
    True
    """
    return (
        f"{advisory} has {count} exception blocks; keep one dated, justified "
        f"block per ignored advisory"
    )


def _undocumented(advisory: str) -> str:
    """Return the fault for an ignore with no block.

    Parameters
    ----------
    advisory : str
        The identifier.

    Returns
    -------
    str
        The message.
    """
    return (
        f"{advisory} is ignored with no dated exception block; add one with an "
        f"expiry and a justification"
    )


def _unused(advisory: str) -> str:
    """Return the fault for a block whose advisory is not ignored.

    Parameters
    ----------
    advisory : str
        The identifier.

    Returns
    -------
    str
        The message.
    """
    return (
        f"{advisory} has an exception block but is not ignored; remove the "
        f"block or restore the ignore"
    )


def _expired(block: Exception_) -> str:
    """Return the fault for an exception whose day has passed.

    Parameters
    ----------
    block : Exception_
        The expired block.

    Returns
    -------
    str
        The message.
    """
    return (
        f"the exception for {block.advisory} expired on "
        f"{block.expires_at.isoformat()}; clear the advisory or re-justify it "
        f"with a new date"
    )


def read_config(path: Path) -> str | None:
    """Return the configuration's text, or None when there is no file.

    Only a missing file means nothing is ignored. A directory, an unreadable
    file or text that is not UTF-8 is a fault: treating it as missing would
    pass the gate on a configuration nobody read.

    Raises
    ------
    AuditExceptionError
        If the path exists but cannot be read as UTF-8 text.

    Examples
    --------
    >>> read_config(Path("/nonexistent/audit.toml")) is None
    True
    """
    try:
        return path.read_text(encoding="utf-8")
    except FileNotFoundError:
        return None
    except (OSError, UnicodeDecodeError) as error:
        message = f"cannot read the audit configuration at {path}: {error}"
        raise AuditExceptionError(message) from error


def main(argv: list[str] | None = None) -> int:
    """Validate the audit exceptions and report every fault.

    Parameters
    ----------
    argv : list[str] or None
        Command-line arguments; the first is the configuration path when
        given, otherwise :data:`AUDIT_CONFIG` is read.

    Returns
    -------
    int
        0 when the exceptions are acceptable, 1 otherwise.
    """
    arguments = sys.argv[1:] if argv is None else argv
    path = Path(arguments[0]) if arguments else AUDIT_CONFIG
    try:
        text = read_config(path)
        if text is None:
            print(f"no audit configuration at {path}; nothing is ignored")
            return 0
        found = faults(text, dt.date.today())
    except AuditExceptionError as error:
        # A malformed block is a finding about the file, not a crash in the
        # reader, so it is reported the way every other fault here is. A
        # traceback would send the reader to this script rather than to the
        # line they have to change.
        print(f"audit exception fault: {error}", file=sys.stderr)
        return 1
    for fault in found:
        print(f"audit exception fault: {fault}", file=sys.stderr)
    if found:
        return 1
    blocks = parse_blocks(text)
    if blocks:
        for block in blocks:
            print(
                f"audit exception: {block.advisory} until "
                f"{block.expires_at.isoformat()} — {block.justification}"
            )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
