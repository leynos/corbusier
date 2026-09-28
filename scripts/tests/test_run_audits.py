"""Tests for the audit runner that runs both halves and reports both.

The runner replaced `audit: audit-node rust-audit`, whose Make prerequisites
stopped at the first failure: 24 frontend advisories hid a Rust advisory for
weeks. These cases drive the runner with halves whose outcome is chosen, so
the rule that one failure must not hide the other is asserted, not assumed.
"""

from __future__ import annotations

import sys
import typing as typ
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from run_audits import (  # noqa: E402 - after the path fixup above
    Half,
    audit_halves,
    main,
    run,
    summarize,
)

#: A half that succeeds and one that fails, without running any audit.
PASSING: typ.Final = Half("passing half", [sys.executable, "-c", "raise SystemExit(0)"])
FAILING: typ.Final = Half("failing half", [sys.executable, "-c", "raise SystemExit(3)"])


@pytest.mark.parametrize(
    ("halves", "expected"),
    [
        pytest.param([PASSING, PASSING], 0, id="both-pass"),
        pytest.param([FAILING, PASSING], 1, id="first-fails"),
        pytest.param([PASSING, FAILING], 1, id="second-fails"),
        pytest.param([FAILING, FAILING], 1, id="both-fail"),
    ],
)
def test_the_exit_status_is_the_worse_of_the_halves(
    halves: list[Half], expected: int
) -> None:
    """Any failing half fails the run; only two passes pass it."""
    assert main(halves) == expected


def test_a_failing_first_half_does_not_stop_the_second(
    capsys: pytest.CaptureFixture[str],
) -> None:
    """The defect the runner exists to fix: both halves run and both report."""
    main([FAILING, PASSING])
    out = capsys.readouterr().out

    assert "=== passing half ===" in out
    assert "FAIL  failing half" in out
    assert "PASS  passing half" in out


def test_a_half_that_cannot_start_fails_without_stopping_the_other(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    """A missing executable fails its half; the other half still runs and reports."""
    missing = Half("missing tool", [str(tmp_path / "no-such-tool")])

    assert main([missing, PASSING]) == 1
    captured = capsys.readouterr()
    assert "FAIL  missing tool" in captured.out
    assert "PASS  passing half" in captured.out
    assert "cannot run" in captured.err


def test_a_hung_half_is_stopped_and_fails(capsys: pytest.CaptureFixture[str]) -> None:
    """A half that outruns its timeout is killed and fails with 124."""
    hung = Half("hung half", [sys.executable, "-c", "import time; time.sleep(30)"])

    assert run(hung, timeout=0.5) == 124
    assert "ran longer than 0.5 s" in capsys.readouterr().err


def test_the_summary_names_each_half_in_order(
    capsys: pytest.CaptureFixture[str],
) -> None:
    """One line per half, in the order the halves ran."""
    assert summarize([("frontend", 0), ("rust", 2)]) == 1
    lines = capsys.readouterr().out.splitlines()

    assert lines[-2:] == ["PASS  frontend", "FAIL  rust"]


@pytest.mark.parametrize(
    ("environ", "expected"),
    [
        pytest.param({"MAKE": "gmake"}, "gmake", id="invoking-make"),
        pytest.param({}, "make", id="unset"),
        pytest.param({"MAKE": ""}, "make", id="empty"),
    ],
)
def test_the_halves_run_under_the_invoking_make(
    environ: dict[str, str], expected: str
) -> None:
    """The recipe passes `$(MAKE)` in, and the runner falls back to `make`.

    The environment is injected, so no test mutates the process's own.
    """
    assert [half.command for half in audit_halves(environ)] == [
        [expected, "audit-node"],
        [expected, "rust-audit"],
    ]
