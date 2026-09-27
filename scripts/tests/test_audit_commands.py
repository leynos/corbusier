"""Behavioural tests for the audit commands, run as the gate runs them.

`test_rust_audit_exceptions` and `test_run_audits` call the functions. These
cases cross the process boundary instead: the validator runs as a script
against temporary configurations, and `make audit` runs the real recipe with a
stand-in `make` whose halves pass or fail on request, so the Makefile wiring,
the runner's exit status and its summary order are all asserted together.
The stand-in is passed as `MAKE` on the command line of a child `make`, so no
test changes this process's environment and no real audit runs.
"""

from __future__ import annotations

import os
import stat
import subprocess
import sys
import textwrap
import typing as typ
from pathlib import Path

import pytest

REPO_ROOT: typ.Final[Path] = Path(__file__).resolve().parents[2]

#: Seconds any child process may run. The stand-in answers at once; a real
#: audit reached by a broken recipe must fail the case, not hang the gate.
CHILD_TIMEOUT: typ.Final[int] = 180
VALIDATOR: typ.Final[Path] = REPO_ROOT / "scripts/rust_audit_exceptions.py"

#: One dated, justified exception block over a matching ignore list, with the
#: expiry supplied per case so no case depends on the day it runs.
CONFIG_TEMPLATE: typ.Final[str] = """\
# advisory: RUSTSEC-2026-0258
# expires-at: {expires}
# justification: the only dependent pins the vulnerable major.

[advisories]
ignore = [{ignore}]
"""


def _config(tmp_path: Path, *, expires: str = "2999-12-31", ignore: str = '"RUSTSEC-2026-0258"') -> Path:
    """Write an audit configuration and return its path."""
    path = tmp_path / "audit.toml"
    path.write_text(CONFIG_TEMPLATE.format(expires=expires, ignore=ignore), encoding="utf-8")
    return path


def _validate(path: Path) -> subprocess.CompletedProcess[str]:
    """Run the validator script on a configuration path, as `make` does."""
    return subprocess.run(  # noqa: S603 - fixed interpreter and script
        [sys.executable, str(VALIDATOR), str(path)],
        capture_output=True,
        text=True,
        check=False,
        timeout=CHILD_TIMEOUT,
    )


def test_the_validator_accepts_a_current_exception(tmp_path: Path) -> None:
    """A dated, justified, matching exception exits 0 and is listed."""
    result = _validate(_config(tmp_path))

    assert result.returncode == 0, result.stderr
    assert "audit exception: RUSTSEC-2026-0258 until 2999-12-31" in result.stdout


def test_the_validator_accepts_a_missing_configuration(tmp_path: Path) -> None:
    """No configuration means nothing is ignored, which is not a fault."""
    result = _validate(tmp_path / "absent.toml")

    assert result.returncode == 0
    assert "nothing is ignored" in result.stdout


@pytest.mark.parametrize(
    ("config", "message"),
    [
        pytest.param({"expires": "2000-01-01"}, "expired on 2000-01-01", id="expired"),
        pytest.param({"expires": "2026-02-30"}, "not a calendar date", id="malformed-date"),
        pytest.param({"ignore": ""}, "has an exception block but is not ignored", id="stale-block"),
        pytest.param(
            {"ignore": '"RUSTSEC-2026-0258", "RUSTSEC-2026-0001"'},
            "RUSTSEC-2026-0001 is ignored with no dated exception block",
            id="undocumented-ignore",
        ),
        pytest.param({"ignore": '"RUSTSEC-2026-0258'}, "not valid TOML", id="unterminated-string"),
    ],
)
def test_the_validator_reports_each_fault_and_exits_1(
    tmp_path: Path, config: dict[str, str], message: str
) -> None:
    """Every fault reaches stderr as a named finding, never a traceback."""
    result = _validate(_config(tmp_path, **config))

    assert result.returncode == 1
    assert "audit exception fault:" in result.stderr
    assert message in result.stderr
    assert "Traceback" not in result.stderr


def _stand_in_make(tmp_path: Path, failing: frozenset[str]) -> Path:
    """Write a `make` stand-in that records each target and fails the named ones."""
    log = tmp_path / "targets.log"
    script = tmp_path / "fake-make"
    script.write_text(
        textwrap.dedent(
            f"""\
            #!{sys.executable}
            import sys
            target = sys.argv[-1]
            with open({str(log)!r}, "a", encoding="utf-8") as handle:
                handle.write(target + "\\n")
            raise SystemExit(1 if target in {sorted(failing)!r} else 0)
            """
        ),
        encoding="utf-8",
    )
    script.chmod(script.stat().st_mode | stat.S_IXUSR)
    return script


def _run_make_audit(fake: Path) -> subprocess.CompletedProcess[str]:
    """Run the real `audit` recipe with the stand-in passed as `MAKE`."""
    environ = {key: value for key, value in os.environ.items() if key not in {"MAKEFLAGS", "MAKELEVEL", "MFLAGS"}}
    return subprocess.run(  # noqa: S603 - fixed command
        ["make", "--no-print-directory", "-C", str(REPO_ROOT), f"MAKE={fake}", "audit"],  # noqa: S607
        capture_output=True,
        text=True,
        check=False,
        env=environ,
        timeout=CHILD_TIMEOUT,
    )


@pytest.mark.parametrize(
    ("failing", "status", "summary"),
    [
        pytest.param(frozenset(), 0, ["PASS  frontend (bun audit)", "PASS  Rust (cargo audit)"], id="both-pass"),
        pytest.param(
            frozenset({"audit-node"}), 2, ["FAIL  frontend (bun audit)", "PASS  Rust (cargo audit)"], id="frontend-fails"
        ),
        pytest.param(
            frozenset({"rust-audit"}), 2, ["PASS  frontend (bun audit)", "FAIL  Rust (cargo audit)"], id="rust-fails"
        ),
    ],
)
def test_make_audit_runs_both_halves_and_reports_both(
    tmp_path: Path, failing: frozenset[str], status: int, summary: list[str]
) -> None:
    """The recipe runs both halves in order, reports both, and fails on either.

    `make` itself exits 2 when a recipe fails, so a failing half surfaces as
    2 from the outer `make` and 1 from the runner inside it.
    """
    result = _run_make_audit(_stand_in_make(tmp_path, failing))

    assert result.returncode == status, result.stdout + result.stderr
    assert (tmp_path / "targets.log").read_text(encoding="utf-8").split() == ["audit-node", "rust-audit"]
    assert result.stdout.splitlines()[-2:] == summary
