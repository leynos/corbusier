"""The dated exception blocks are load-bearing, not decorative.

`cargo-audit` takes a bare list of advisory identifiers and rejects any key it
does not know, so an expiry and a justification cannot be fields of
`.cargo/audit.toml`. They are comments, and a comment enforces nothing on its
own: without this contract an ignore could be added with no date at all, or
left in place years after its cause was fixed, and the file would read exactly
as it does now.

The rule is driven with constructed configurations as well as the real one,
because this repository's own file is correct: parametrized over it alone, a
reader that returned no faults whatever it was given would pass.
"""

from __future__ import annotations

import datetime as dt
import sys
import typing as typ
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from rust_audit_exceptions import (  # noqa: E402 - after the path fixup above
    AuditExceptionError,
    faults,
    ignored_advisories,
    parse_blocks,
)

REPO_ROOT: typ.Final[Path] = Path(__file__).resolve().parents[2]

#: A well-formed configuration, as a template the cases perturb.
GOOD: typ.Final[str] = """\
# advisory: RUSTSEC-2026-0258
# expires-at: 2026-12-17
# justification: the only dependent pins the vulnerable major.

[advisories]
ignore = ["RUSTSEC-2026-0258"]
"""

#: The date the constructed cases are judged against.
TODAY: typ.Final[dt.date] = dt.date(2026, 9, 17)


def test_the_real_configuration_is_acceptable_today() -> None:
    """The file in the tree passes its own rule.

    Asserted separately from the constructed cases so that a change to the
    real file which breaks the rule fails here by name, rather than only when
    someone runs the audit.
    """
    text = (REPO_ROOT / ".cargo/audit.toml").read_text(encoding="utf-8")

    assert faults(text, dt.date.today()) == []


def test_every_ignored_advisory_is_documented_and_dated() -> None:
    """The ignore list and the blocks describe the same set.

    Both directions matter and read as each other from the file alone. An
    ignore with no block is an undocumented exception; a block with no ignore
    is a justification left behind for something the audit no longer
    suppresses, which is how a stale reason comes to look like a current one.
    """
    ignored = ignored_advisories(GOOD)
    documented = [block.advisory for block in parse_blocks(GOOD)]

    assert ignored == documented
    assert faults(GOOD, TODAY) == []


def test_an_undated_exception_is_refused() -> None:
    """An exception that cannot expire is not a deferral.

    This is the mutation the contract exists for: deleting the expiry line
    leaves a file `cargo-audit` still accepts and a justification that still
    reads well.
    """
    undated = GOOD.replace("# expires-at: 2026-12-17\n", "")

    with pytest.raises(AuditExceptionError, match=r"names no expiry"):
        faults(undated, TODAY)


def test_an_unjustified_exception_is_refused() -> None:
    """A date without a reason says when to look again and not what at."""
    unjustified = GOOD.replace(
        "# justification: the only dependent pins the vulnerable major.\n", ""
    )

    with pytest.raises(AuditExceptionError, match=r"no justification"):
        faults(unjustified, TODAY)


@pytest.mark.parametrize(
    ("today", "expected"),
    [
        pytest.param(dt.date(2026, 12, 16), False, id="the-day-before"),
        pytest.param(dt.date(2026, 12, 17), False, id="the-day-itself"),
        pytest.param(dt.date(2026, 12, 18), True, id="the-day-after"),
    ],
)
def test_the_expiry_is_inclusive_through_its_own_day(
    today: dt.date, expected: bool
) -> None:
    """The named day is still covered; the one after it is not.

    The boundary is asserted from both sides because an off-by-one here is
    invisible: a rule that expired a day early and one that expired a day late
    both look like "it expires in December".
    """
    found = faults(GOOD, today)

    assert bool(found) is expected, f"{today} gave {found}"
    if expected:
        assert "expired on 2026-12-17" in found[0]


def test_an_ignore_with_no_block_is_a_fault() -> None:
    """An undocumented ignore is the case the blocks exist to prevent."""
    undocumented = GOOD.replace(
        'ignore = ["RUSTSEC-2026-0258"]',
        'ignore = ["RUSTSEC-2026-0258", "RUSTSEC-2026-0097"]',
    )

    found = faults(undocumented, TODAY)

    assert len(found) == 1, found
    assert "RUSTSEC-2026-0097 is ignored with no dated exception block" in found[0]


def test_a_block_with_no_ignore_is_a_fault() -> None:
    """A justification for something no longer suppressed is stale."""
    stale = GOOD.replace('ignore = ["RUSTSEC-2026-0258"]', "ignore = []")

    found = faults(stale, TODAY)

    assert len(found) == 1, found
    assert "has an exception block but is not ignored" in found[0]
