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

import datetime as dt
import re
import sys
import tomllib
import typing as typ
from pathlib import Path

#: The audit configuration, relative to the repository root.
AUDIT_CONFIG: typ.Final[Path] = Path(".cargo/audit.toml")

#: A comment line opening one exception block.
_ADVISORY = re.compile(r"^#\s*advisory:\s*(?P<id>RUSTSEC-\d{4}-\d{4})\s*$")

#: The expiry line of a block, an ISO date on its own.
_EXPIRES = re.compile(r"^#\s*expires-at:\s*(?P<date>\d{4}-\d{2}-\d{2})\s*$")

#: The first line of a justification, which may continue on indented lines.
_JUSTIFICATION = re.compile(r"^#\s*justification:\s*(?P<text>\S.*)$")


class AuditExceptionError(ValueError):
    """Raised when the exception blocks and the ignore list disagree."""


class Exception_(typ.NamedTuple):
    """One dated exception block.

    Attributes
    ----------
    advisory : str
        The RUSTSEC identifier.
    expires_at : datetime.date
        The last day the exception is honoured, inclusive.
    justification : str
        Why the advisory cannot be cleared here.
    """

    advisory: str
    expires_at: dt.date
    justification: str


def parse_blocks(text: str) -> list[Exception_]:
    """Return the exception blocks the header declares, in file order.

    Parameters
    ----------
    text : str
        The audit configuration's text.

    Returns
    -------
    list[Exception_]
        One entry per block.

    Raises
    ------
    AuditExceptionError
        If a block names an advisory and then omits its expiry or its
        justification, or gives an expiry that is not an ISO date.
    """
    blocks: list[Exception_] = []
    lines = text.splitlines()
    for index, line in enumerate(lines):
        opened = _ADVISORY.match(line)
        if opened is None:
            continue
        advisory = opened.group("id")
        expires = _read_expiry(lines, index + 1, advisory)
        justification = _read_justification(lines, index + 1, advisory)
        blocks.append(Exception_(advisory, expires, justification))
    return blocks


def _parse_expiry(raw: str, advisory: str) -> dt.date:
    """Return the date an `expires-at` line names.

    The line's pattern accepts any three digit groups, so `2026-13-45`
    reaches here. It is refused as a fault naming the line to change rather
    than escaping as a bare `ValueError` that `main` does not catch.

    Raises
    ------
    AuditExceptionError
        If the digits do not form a calendar date.

    Examples
    --------
    >>> _parse_expiry("2026-12-17", "RUSTSEC-2026-0258")
    datetime.date(2026, 12, 17)
    """
    try:
        return dt.date.fromisoformat(raw)
    except ValueError as error:
        message = (
            f"the exception for {advisory} names `{raw}`, which is not a "
            f"calendar date; write the expiry as YYYY-MM-DD"
        )
        raise AuditExceptionError(message) from error


def _read_expiry(lines: list[str], start: int, advisory: str) -> dt.date:
    """Return the expiry date following an advisory line.

    Parameters
    ----------
    lines : list[str]
        The file's lines.
    start : int
        Index of the line after the advisory line.
    advisory : str
        The identifier, for the message.

    Returns
    -------
    datetime.date
        The expiry.

    Raises
    ------
    AuditExceptionError
        If the next non-blank comment line is not an expiry, or names a
        date that is not on the calendar.
    """
    for line in lines[start:]:
        found = _EXPIRES.match(line)
        if found is not None:
            return _parse_expiry(found.group("date"), advisory)
        if _ADVISORY.match(line) or not line.startswith("#"):
            break
    message = (
        f"the exception for {advisory} names no expiry; add an "
        f"`# expires-at: YYYY-MM-DD` line, because an exception that cannot "
        f"expire is not a deferral"
    )
    raise AuditExceptionError(message)


def _read_justification(lines: list[str], start: int, advisory: str) -> str:
    """Return the justification following an advisory line.

    Parameters
    ----------
    lines : list[str]
        The file's lines.
    start : int
        Index of the line after the advisory line.
    advisory : str
        The identifier, for the message.

    Returns
    -------
    str
        The justification, continuation lines joined with spaces.

    Raises
    ------
    AuditExceptionError
        If no justification line follows.
    """
    collected: list[str] = []
    for line in lines[start:]:
        if collected:
            if _ADVISORY.match(line) or not line.startswith("#"):
                break
            continuation = line.lstrip("#").strip()
            if not continuation or _EXPIRES.match(line):
                break
            collected.append(continuation)
            continue
        opened = _JUSTIFICATION.match(line)
        if opened is not None:
            collected.append(opened.group("text").strip())
            continue
        if _ADVISORY.match(line) or not line.startswith("#"):
            break
    if collected:
        return " ".join(collected)
    message = (
        f"the exception for {advisory} carries no justification; add an "
        f"`# justification:` line saying why the advisory cannot be cleared "
        f"here"
    )
    raise AuditExceptionError(message)


def ignored_advisories(text: str) -> list[str]:
    """Return the advisories `cargo-audit` is told to ignore.

    Parameters
    ----------
    text : str
        The audit configuration's text.

    Returns
    -------
    list[str]
        The identifiers, in file order.
    """
    parsed = tomllib.loads(text)
    advisories = parsed.get("advisories")
    if not isinstance(advisories, dict):
        return []
    ignore = advisories.get("ignore")
    return [str(entry) for entry in ignore] if isinstance(ignore, list) else []


def faults(text: str, today: dt.date) -> list[str]:
    """Return every reason the exceptions are not acceptable, in order.

    Both directions are checked. An ignored advisory with no block is an
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
    documented = {block.advisory: block for block in parse_blocks(text)}
    ignored = ignored_advisories(text)
    undocumented = [
        _undocumented(advisory) for advisory in ignored if advisory not in documented
    ]
    unused = [_unused(advisory) for advisory in documented if advisory not in ignored]
    expired = [
        _expired(block)
        for advisory, block in documented.items()
        if advisory in ignored and block.expires_at < today
    ]
    return [*undocumented, *unused, *expired]


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
    if not path.exists():
        print(f"no audit configuration at {path}; nothing is ignored")
        return 0
    text = path.read_text(encoding="utf-8")
    try:
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
