---
name: impeccable
description: Retired Impeccable entry, kept for one migration major version. Use only when the user explicitly invokes the old impeccable command; it names the Phoenix UI replacement and does nothing else.
disable-model-invocation: true
---

# Impeccable is now XGENT Phoenix UI

This project was migrated from Impeccable to XGENT Phoenix UI. This entry only points the old command at the new tool. It does not run the old tool, install hooks or trigger automatically, and it will be removed in the major version after the first migration major version.

Tell the user, in their language:

- Design commands are now `{{prefix}}phoenix-ui <command>`. For example, `{{prefix}}impeccable polish` becomes `{{prefix}}phoenix-ui polish`, and `{{prefix}}impeccable init` becomes `{{prefix}}phoenix-ui init`.
- Project state moved from `.impeccable/` to `.phoenix-ui/`. The old directory is kept unchanged as a recovery source.
- The installer printed a rollback command (`phoenix-ui migrate --rollback <transaction> --project <dir>`) that restores the previous entry points.

Then stop. Do not run an `impeccable` executable, and do not continue the old request under the new name unless the user asks you to.
