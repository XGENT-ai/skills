# Features

## Core Sections (Required)

### 1) Feature Inventory

List every user- or caller-facing capability found in routes, pages, CLI commands, menus, or permission definitions. Group by domain.

| Feature | What it does (observable behavior) | Entry point (route/page/command) | Core logic | Data touched | Evidence |
|---------|-----------------------------------|----------------------------------|-----------|--------------|----------|
| [name] | [behavior] | [route or page] | [path] | [tables/services] | [file] |

Total: [N] features across [M] domains.

### 2) Key Feature Walkthroughs

Pick the 3-5 most important features (most used, most changed, or most complex). For each:

- Trigger: [who does what, where]
- Path through the code: [entry file:line] -> [service file:line] -> [data/integration file:line]
- Side effects: [events, notifications, audit logs, external calls]
- Failure modes: [what can go wrong and how it surfaces to the user]

### 3) Switches That Change Behavior

| Switch | Type (feature flag/tenant setting/plan limit/env var) | Affects | Default | Evidence |
|--------|------------------------------------------------------|---------|---------|----------|
| [name] | [type] | [feature] | [value] | [file] |

### 4) Evidence

- [path/to/route-or-page-registry]
- [path/to/permission-or-menu-definition]
- [path/to/key-service-file]

## Extended Sections (Optional)

Add only when needed:

- Role/permission x feature matrix
- Features declared but not wired to an entry point (dead or unfinished)
- Feature-level dependency map
- Per-feature test coverage notes
