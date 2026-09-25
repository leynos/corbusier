#!/usr/bin/env -S uv run python
# /// script
# requires-python = ">=3.13"
# dependencies = []
# ///
"""Run both dependency audits and report both outcomes.

`audit: audit-node rust-audit` made the two halves Make prerequisites, so a
failing frontend audit stopped the target before the Rust half ran. That is
how this repository came to carry two audit failures while only one was
visible: 24 frontend advisories masked RUSTSEC-2026-0258 entirely, and
clearing the frontend revealed a Rust advisory that had been there since
2026-08-17.

One failure must not hide another, so both halves always run and both report,
and the exit status is the worse of the two.
"""

from __future__ import annotations

import os
import subprocess  # noqa: S404 - the commands are fixed, not caller-supplied
import typing as typ


class Half(typ.NamedTuple):
    """One audit half.

    Attributes
    ----------
    name : str
        What to call it in the summary.
    command : list[str]
        The command to run.
    """

    name: str
    command: list[str]


def run(half: Half) -> int:
    """Run one half and return its exit status.

    Output is not captured. A gate's output is what makes a failure
    actionable, and holding it back to re-print later loses the interleaving
    with the tool's own progress.

    Parameters
    ----------
    half : Half
        The half to run.

    Returns
    -------
    int
        The command's exit status.
    """
    print(f"\n=== {half.name} ===", flush=True)
    return subprocess.call(half.command)  # noqa: S603 - fixed command


def summarize(results: list[tuple[str, int]]) -> int:
    """Print one line per half and return the status to exit with.

    Parameters
    ----------
    results : list of tuple
        Each half's name and exit status, in the order they ran.

    Returns
    -------
    int
        0 when every half passed, otherwise 1.
    """
    print("\n=== audit summary ===", flush=True)
    for name, status in results:
        print(f"{'PASS' if status == 0 else 'FAIL'}  {name}")
    return 0 if all(status == 0 for _, status in results) else 1


def main(halves: list[Half] | None = None) -> int:
    """Run every half, report both, and fail if either failed.

    Parameters
    ----------
    halves : list[Half] or None
        The halves to run; the two real ones by default.

    Returns
    -------
    int
        0 when every half passed, otherwise 1.
    """
    to_run = HALVES if halves is None else halves
    results = [(half.name, run(half)) for half in to_run]
    return summarize(results)


#: The make executable that invoked this runner. The `audit` recipe passes
#: `$(MAKE)` in, so a wrapper or an alternate make such as `gmake` runs the
#: halves too; run by hand, it falls back to `make`.
MAKE: typ.Final[str] = os.environ.get("MAKE") or "make"

#: The two halves, in the order they run.
HALVES: typ.Final[list[Half]] = [
    Half("frontend (bun audit)", [MAKE, "audit-node"]),
    Half("Rust (cargo audit)", [MAKE, "rust-audit"]),
]


if __name__ == "__main__":
    raise SystemExit(main())
