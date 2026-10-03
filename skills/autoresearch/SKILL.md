---
name: autoresearch
description: 'Use for measured, iterative optimization; skip one-shot edits and review-only tasks. Routes to agi-mode Hillclimb for experiments on code, skills, prompts or agent workflows.'
license: MIT
compatibility: Requires agi-mode to be installed alongside this entry and a runnable measurement path.
---

# Autoresearch

The maintained workflow now lives in **agi-mode**. Read and execute [Hillclimb](../agi-mode/playbooks/hillclimb.md); it owns setup, baseline, safe recovery, experiment records, acceptance, plateaus and final confirmation. For behavioral comparisons it routes to [Eval](../agi-mode/playbooks/eval.md).

Reuse the user's existing goal, scope and permissions. This entry does not create a separate loop, mandate commits, or imply unlimited resources. Git is optional when equivalent recoverable snapshots are available.

Resolve the sibling path or the registered installation of `agi-mode`. If it is absent, explain that this compatibility entry requires `agi-mode`; do not invent a second implementation or claim to have run the missing workflow.
