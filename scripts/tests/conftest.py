"""Put ``scripts/`` on the import path for the tests in this directory.

The modules under test live in ``scripts/``, which is not a package and is
not on ``sys.path`` when pytest collects from the repository root. Doing it
here rather than in each module keeps the imports at the top of each file,
where ``E402`` expects them, instead of after a path fixup that a reader has
to notice before the imports make sense.

``scripts/tests/test_typos_rollout.py`` is invoked with ``PYTHONPATH=scripts``
from its own Make target and does not rely on this.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
