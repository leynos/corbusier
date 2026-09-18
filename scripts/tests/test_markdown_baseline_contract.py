"""Contracts on the Markdown formatting baseline (concordat #168).

Two things can drift apart and neither shows up as a failing build: the
ignore list in `.markdownlint-cli2.jsonc`, which both CI and
`make markdownlint` consult, and the way CI invokes the linter. A
negation in the workflow's globs block is invisible to the local gate, so
the two would disagree about which documents are linted while both stayed
green. A second invocation of markdownlint from anywhere else in CI would
lint through whatever version happened to be on the runner rather than
through the pinned action.

Each assertion here is written against the thing that would break, not
against the wording of the rule, and each was proved by putting the
forbidden element back.
"""

from __future__ import annotations

import json
import re
import typing as typ
from pathlib import Path

import pytest
import yaml

REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
CONFIG_PATH = REPOSITORY_ROOT / ".markdownlint-cli2.jsonc"
WORKFLOW_DIRECTORY = REPOSITORY_ROOT / ".github" / "workflows"

#: The ignore entries the estate baseline requires of every repository.
#:
#: A repository may carry more (corbusier keeps `.node_modules/**` beside
#: the `**/`-prefixed superset), but never fewer: a document moved between
#: repositories must keep its lint status.
CANONICAL_IGNORES: typ.Final = (
    "**/.venv/**",
    ".vtcode/**",
    "**/node_modules/**",
    "**/target/**",
    ".terraform/**",
    ".uv-cache/**",
    "memories/**",
    "CRUSH.md",
)

#: The pinned markdownlint-cli2 action, owner and repository folded because
#: only those are case-insensitive to GitHub.
MARKDOWNLINT_ACTION: typ.Final = "davidanson/markdownlint-cli2-action"

#: v24.2.0, as a commit rather than the annotated tag object PD-006 rules
#: out. Read from the GitHub API rather than assumed: the previous pin,
#: 992badcd, is v20.0.0.
MARKDOWNLINT_ACTION_REF: typ.Final = "21c1be1b93ad9ed58fa840aacc3f279cde2a72ff"

_COMMENT = re.compile(r"^\s*//.*$", flags=re.MULTILINE)


def _config() -> dict[str, typ.Any]:
    """Return the parsed markdownlint configuration.

    Returns
    -------
    dict
        The configuration, with `//` comment lines stripped so that the
        JSON-with-comments file parses as JSON.
    """
    text = _COMMENT.sub("", CONFIG_PATH.read_text(encoding="utf-8"))
    return json.loads(text)


def _workflows() -> list[tuple[Path, dict[str, typ.Any]]]:
    """Return every workflow document under `.github/workflows`.

    Returns
    -------
    list of (Path, dict)
        Each workflow file and its parsed document.
    """
    found = []
    for path in sorted(WORKFLOW_DIRECTORY.glob("*.yml")):
        found.append((path, yaml.safe_load(path.read_text(encoding="utf-8"))))
    assert found, "no workflow documents were collected"
    return found


def _steps(document: dict[str, typ.Any]) -> typ.Iterator[dict[str, typ.Any]]:
    """Yield every step of every job in one workflow document.

    Parameters
    ----------
    document : dict
        A parsed workflow.

    Yields
    ------
    dict
        One step mapping.
    """
    for job in (document.get("jobs") or {}).values():
        for step in job.get("steps") or []:
            if isinstance(step, dict):
                yield step


@pytest.mark.parametrize("ignore", CANONICAL_IGNORES)
def test_the_config_carries_every_canonical_ignore(ignore: str) -> None:
    """Each canonical ignore is present, by exact string.

    Parametrised one entry per case rather than compared as a set, so a
    failure names the entry that went missing instead of printing two
    lists and leaving the reader to diff them.
    """
    ignores = _config().get("ignores")
    assert isinstance(ignores, list), "the config must declare an ignores list"
    assert ignore in ignores, (
        f"{ignore} is missing from .markdownlint-cli2.jsonc; the baseline "
        f"requires it so a document keeps its lint status across repositories"
    )


def test_ci_lints_markdown_only_through_the_pinned_action() -> None:
    """Every markdownlint invocation in CI is the pinned action.

    Both halves matter. A step running `markdownlint-cli2` through `run:`
    would lint with whatever version the runner supplies, and a second
    `uses:` of the action at another revision would lint the same files
    twice under two rule sets. So the action steps are counted and their
    refs checked, and every `run:` line is searched for the command.
    """
    action_refs = []
    for path, document in _workflows():
        for step in _steps(document):
            uses = step.get("uses")
            if isinstance(uses, str):
                name, _, ref = uses.partition("@")
                if name.lower() == MARKDOWNLINT_ACTION:
                    action_refs.append((path.name, ref))
            script = step.get("run")
            if isinstance(script, str):
                for line in script.splitlines():
                    assert "markdownlint" not in line, (
                        f"{path.name} runs markdownlint through a shell command "
                        f"({line.strip()!r}); it must go through the pinned "
                        f"action so the version is not the runner's choice"
                    )

    assert action_refs, "no workflow uses the markdownlint-cli2 action"
    for name, ref in action_refs:
        assert ref == MARKDOWNLINT_ACTION_REF, (
            f"{name} pins the markdownlint action at {ref}; the baseline "
            f"pins {MARKDOWNLINT_ACTION_REF}"
        )


def test_the_markdown_globs_carry_no_negations() -> None:
    """The workflow's globs exclude nothing of their own.

    A `!`-prefixed glob here is invisible to `make markdownlint`, which
    reads the config alone, so CI and the local gate would lint different
    sets while both reported success. Exclusions belong in the config.
    """
    for path, document in _workflows():
        for step in _steps(document):
            uses = step.get("uses")
            if not isinstance(uses, str):
                continue
            if uses.partition("@")[0].lower() != MARKDOWNLINT_ACTION:
                continue
            globs = (step.get("with") or {}).get("globs", "")
            negations = [
                line.strip()
                for line in str(globs).splitlines()
                if line.strip().startswith("!")
            ]
            assert not negations, (
                f"{path.name} excludes {negations} in the action's globs; put "
                f"those entries in .markdownlint-cli2.jsonc instead, where the "
                f"local gate also reads them"
            )
