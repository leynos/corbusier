"""Read the dated exception blocks and the ignore list in `.cargo/audit.toml`.

The reading half of the audit-exception rule. `rust_audit_exceptions` holds
the rule and the command-line entry point; this module only turns the file's
text into blocks and identifiers, raising `AuditExceptionError` for a block or
a file it cannot read. Split from the rule to keep each module under the
400-line limit.
"""

from __future__ import annotations

import datetime as dt
import re
import tomllib
import typing as typ

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

    Raises
    ------
    AuditExceptionError
        If the file is not valid TOML. `cargo-audit` would refuse it too,
        so it is reported as a fault naming the parser's position rather
        than escaping as a `TOMLDecodeError` that `main` does not catch.
    """
    try:
        parsed = tomllib.loads(text)
    except tomllib.TOMLDecodeError as error:
        message = f"the audit configuration is not valid TOML: {error}"
        raise AuditExceptionError(message) from error
    advisories = parsed.get("advisories")
    if not isinstance(advisories, dict):
        return []
    ignore = advisories.get("ignore")
    return [str(entry) for entry in ignore] if isinstance(ignore, list) else []
