# STAMP migration agents

Project-scoped Claude Code subagents that specialize in the Python 2→3 / PyQt4→6 / numpy 2 /
matplotlib 3 / biom 2.1 migration. They travel with the repo, so they work on the VM too.

Invoke one by mentioning it, e.g. *"use the migration-bug-hunter to sweep `stamp/plugins/multiGroups`"*,
or let Claude route to it automatically based on the task.

| Agent | What it does |
|-------|--------------|
| **migration-bug-hunter** | Greps the tree for the known runtime bug classes and fixes them; verifies with byte-compile + smoke-test. Start here when porting more code or after a crash. |
| **gui-verifier** | Runs and extends the headless offscreen regression harness (`tools/py313_smoketest.py`); reports OK/FAIL per action. Use to reproduce action crashes and confirm fixes without a display. |
| **import-wizard-tester** | Drives the file-import wizards (MG-RAST / Mothur / RITA / CoMet / BIOM / Append-COG) against `examples/` inputs — the paths the GUI harness can't reach. |
| **numerical-regression** | Guards against silent numeric drift (true-division, dtype, RNG); builds/refreshes a baseline and diffs against it. |

**Typical loop:** `migration-bug-hunter` fixes → `gui-verifier` / `import-wizard-tester`
confirm the fix runs → `numerical-regression` confirms the numbers didn't move.

Prereqs: the `.venv313` environment (run `./setup_py313.sh` once). Full context is in
`PYTHON313_SETUP.md` and `MIGRATION_NOTES.md`; recurring bug patterns are also in Claude's
project memory.
