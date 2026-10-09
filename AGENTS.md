# AGENTS.md

Behavioral guidelines to reduce common LLM coding mistakes. Merge with project-specific instructions as needed.

**Tradeoff:** These guidelines bias toward caution over speed. For trivial tasks, use judgment.

## 1. Think Before Coding

**Don't assume. Don't hide confusion. Surface tradeoffs.**

Before implementing:
- State your assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them - don't pick silently.
- If a simpler approach exists, say so. Push back when warranted.
- If something is unclear, stop. Name what's confusing. Ask.

## 2. Simplicity First

**Minimum code that solves the problem. Nothing speculative.**

- No features beyond what was asked.
- No abstractions for single-use code.
- No "flexibility" or "configurability" that wasn't requested.
- No error handling for impossible scenarios.
- If you write 200 lines and it could be 50, rewrite it.

Ask yourself: "Would a senior engineer say this is overcomplicated?" If yes, simplify.

## 3. Surgical Changes

**Touch only what you must. Clean up only your own mess.**

When editing existing code:
- Don't "improve" adjacent code, comments, or formatting.
- Don't refactor things that aren't broken.
- Match existing style, even if you'd do it differently.
- If you notice unrelated dead code, mention it - don't delete it.

When your changes create orphans:
- Remove imports/variables/functions that YOUR changes made unused.
- Don't remove pre-existing dead code unless asked.

The test: Every changed line should trace directly to the user's request.

## 4. Goal-Driven Execution

**Define success criteria. Loop until verified.**

Transform tasks into verifiable goals:
- "Add validation" → "Write tests for invalid inputs, then make them pass"
- "Fix the bug" → "Write a test that reproduces it, then make it pass"
- "Refactor X" → "Ensure tests pass before and after"

For multi-step tasks, state a brief plan:
```
1. [Step] → verify: [check]
2. [Step] → verify: [check]
3. [Step] → verify: [check]
```

Strong success criteria let you loop independently. Weak criteria ("make it work") require constant clarification.

## 5. Evidence & Quotation

**Quote only what you have read. Never write what you haven't.**

- Before quoting a file's exact wording, numbers, identifiers, statuses, commands, or dates, read that passage yourself. Don't reconstruct it from context, and don't present "probably so" as fact.
- When a tool result is truncated (long files echo only head and tail), the middle is **unknown**. Re-read it first (offset / grep / obs_recall), then quote.
- Every "X says Y" in a durable artifact (code review, plan, runbook, commit message, factual claim to the user) must have both X (file and passage) and Y (content) pointable on the spot.
- When a quote contradicts the source: search the whole repo for other copies of the same mistake, fix them all, and tell the owner. Never leave two conflicting versions.

## 6. Skill Descriptions

**This is a skills repository. Make skill selection accurate even when descriptions are truncated.**

- Write each `SKILL.md` description in this order: trigger conditions and essential exclusions, then the core outcome.
- Target at most 200 characters overall, with Chinese descriptions around 100 characters. The first 100 characters should independently convey the applicability boundary. These are repository writing targets, not universal coding-agent truncation limits.
- Put exclusions that prevent likely misrouting next to the trigger, for example, "write/review plans; not for executing an existing plan." Never bury them at the end of a long description or only in the body.
- Keep procedures, exhaustive error-code lists, tool details and duplicate bilingual explanations in the body or references. Descriptions should help select the skill, not enumerate its workflow.
- After editing, read both the full description and its first 100 characters. Preserve intent and meaningful boundaries; do not shorten by mechanically cutting text. Validate YAML with `npm test` and confirm unrelated frontmatter and instructions remain intact. Generic validators such as Codex's `quick_validate.py` reject `argument-hint` and `disable-model-invocation`, which this repository uses on purpose.

---

**These guidelines are working if:** fewer unnecessary changes in diffs, fewer rewrites due to overcomplication, and clarifying questions come before implementation rather than after mistakes.

## Experience Capture

During development, when a non-obvious failure, user correction, or repeated automatable workflow provides a useful lesson, use [experience-capture](skills/experience-capture/SKILL.md) at a natural pause. Save only a few lines from the current context and resume immediately; do not investigate, fetch evidence, reload full records, or interrupt the main task to collect. Skip capture if no suitable pause exists, the task is read-only, or collection is disabled. Deeper investigation belongs to the user's later `$agi-mode 复盘` or `$agi-mode reflect`.

## Working in This Repository

- Run the lightweight Node/Python/metadata tests with `npm test`. Phoenix runtime changes also require `npm run test:phoenix` (mise, Rust 1.99.0 and the pinned build tools are required; missing tools fail instead of skipping). Browser, native platform and real provider-session acceptance must be run and recorded separately; neither command proves those checks passed.
- `skills/<name>/` is the source of truth. `.claude/skills/`, `.agents/skills/` and `.codex/skills/` are gitignored local installs: a skills.sh copy picks up local changes only after push and reinstall, while a symlink to `skills/<name>` always does. When a loaded skill's base directory is an installed copy and the task depends on recent changes, run `diff -rq .claude/skills/<name> skills/<name>` first and follow `skills/<name>` if they differ. Point behavior checks at `skills/<name>/...` paths.
- Changing a skill's name, arguments, next-step commands, verdicts or status words changes an interface. In the same change, update every place that restates it: README tables and loop description; `skills/agi-mode/playbooks/sdlc.md`, `what-next.md` and `evals/scenarios.md`, plus `assets/sdlc/state-model.md` and `protocol.md` when verdicts or report types change; the skill's `agents/openai.yaml`; and rules copied into agi-mode such as `references/interface-design.md`. Find them with `grep -rn '<skill-name>' skills README.md docs`. The test is whether each place still fully describes the new behavior, not whether it conflicts. Leave historical records and snapshot docs as they are, and don't add `agents/openai.yaml` where none exists.
- When creating or revising a skill, follow [Skill 编写与修订](skills/agi-mode/playbooks/authoring-a-skill.md). agi-mode is not auto-invoked (`disable-model-invocation`), so this playbook is not loaded otherwise.
