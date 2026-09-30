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


@pytest.mark.parametrize(
    ("content", "make_directory"),
    [
        pytest.param(None, True, id="directory"),
        pytest.param(b"\xff\xfe not utf-8", False, id="not-utf-8"),
    ],
)
def test_an_unreadable_configuration_is_a_fault_not_a_pass(
    tmp_path: Path, content: bytes | None, make_directory: bool
) -> None:
    """Only a missing file means nothing is ignored; a read failure fails."""
    path = tmp_path / "audit.toml"
    if make_directory:
        path.mkdir()
    else:
        path.write_bytes(content or b"")

    result = _validate(path)

    assert result.returncode == 1
    assert "cannot read the audit configuration" in result.stderr
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
        ["make", "--no-print-directory", "-C", str(REPO_ROOT), f"MAKE={fake}", "audit"],  # noqa: S607 - `make` from PATH is the gate under test
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


def _recorder(tmp_path: Path, name: str, status: int = 0, *, record: str = "name") -> Path:
    """Write a stand-in command that logs a line and exits with `status`.

    `record` is the expression the line is built from: the command's own name
    by default, or the directory it ran in with ``"os.getcwd()"``.
    """
    log = tmp_path / ("order.log" if record == "name" else f"{name}.log")
    line = repr(name) if record == "name" else record
    script = tmp_path / name
    script.write_text(
        f"#!{sys.executable}\n"
        "import os\n"
        f"with open({str(log)!r}, 'a', encoding='utf-8') as handle:\n"
        f"    handle.write({line} + '\\n')\n"
        f"raise SystemExit({status})\n",
        encoding="utf-8",
    )
    script.chmod(script.stat().st_mode | stat.S_IXUSR)
    return script


def _run_rust_audit(
    tmp_path: Path, *, validator_status: int = 0, cargo: Path, workspace: Path | None = None
) -> subprocess.CompletedProcess[str]:
    """Run the real `rust-audit` recipe with stand-ins for pytest, the validator and cargo.

    Run from the repository by default, or from `workspace` with this
    repository's Makefile, so the `find` in the recipe searches that tree.
    """
    directory = ["-C", str(workspace), "-f", str(REPO_ROOT / "Makefile")] if workspace else ["-C", str(REPO_ROOT)]
    environ = {key: value for key, value in os.environ.items() if key not in {"MAKEFLAGS", "MAKELEVEL", "MFLAGS"}}
    return subprocess.run(  # noqa: S603 - fixed command
        [  # noqa: S607 - `make` from PATH is the gate under test
            "make", "--no-print-directory", *directory,
            f"AUDIT_PYTEST={_recorder(tmp_path, 'pytest')}",
            f"AUDIT_EXCEPTIONS={_recorder(tmp_path, 'validator', validator_status)}",
            f"CARGO={cargo}",
            "rust-audit",
        ],
        capture_output=True,
        text=True,
        check=False,
        env=environ,
        timeout=CHILD_TIMEOUT,
    )


@pytest.mark.parametrize(
    ("validator_status", "expected_order", "make_status"),
    [
        pytest.param(0, ["pytest", "validator", "cargo"], 0, id="exceptions-pass"),
        pytest.param(1, ["pytest", "validator"], 2, id="exceptions-fail"),
    ],
)
def test_rust_audit_runs_the_exception_gate_first_and_stops_on_it(
    tmp_path: Path, validator_status: int, expected_order: list[str], make_status: int
) -> None:
    """`rust-audit` runs the exception tests and validator before `cargo audit`.

    With the validator failing, `cargo audit` never runs and `make` fails, so
    removing `audit-exceptions` from the target's prerequisites fails here.
    """
    result = _run_rust_audit(
        tmp_path, validator_status=validator_status, cargo=_recorder(tmp_path, "cargo")
    )

    assert result.returncode == make_status, result.stdout + result.stderr
    assert (tmp_path / "order.log").read_text(encoding="utf-8").split() == expected_order


@pytest.mark.parametrize(
    "pruned",
    [
        pytest.param("target/debug/build", id="target"),
        pytest.param("frontend/node_modules/pkg", id="node-modules"),
        pytest.param(".venv/lib/pkg", id="venv"),
        pytest.param(".uv-cache/git-v0/checkouts/abc/fixture", id="uv-cache"),
        pytest.param(".uv-tools/tool/fixture", id="uv-tools"),
    ],
)
def test_rust_audit_reaches_the_workspace_manifest_and_prunes_tool_trees(
    tmp_path: Path, pruned: str
) -> None:
    """`rust-audit` audits the workspace manifest and skips vendored trees.

    The contract target runs the shared library from a checkout in the
    repository-local uv cache, and that checkout holds fixture manifests with
    no targets, which `cargo audit` refuses. A real workspace manifest and a
    decoy under each pruned directory are created in a scratch tree, the recipe
    runs there with a stand-in `cargo` that logs where it ran, and only the
    workspace directory may be logged, so dropping any prune fails its case.
    """
    workspace = tmp_path / "workspace"
    (workspace / "crates/app").mkdir(parents=True)
    (workspace / "Cargo.toml").write_text("[workspace]\n", encoding="utf-8")
    decoy = workspace / pruned
    decoy.mkdir(parents=True)
    (decoy / "Cargo.toml").write_text("[package]\n", encoding="utf-8")
    result = _run_rust_audit(
        tmp_path, cargo=_recorder(tmp_path, "cargo", record="os.getcwd()"), workspace=workspace
    )

    assert result.returncode == 0, result.stdout + result.stderr
    audited = (tmp_path / "cargo.log").read_text(encoding="utf-8").split()
    assert audited == [str(workspace)], f"only the workspace manifest may be audited, saw {audited}"
