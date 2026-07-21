# Product

## Register

product

## Users

A single role-based portal serving two audiences at once:

- **Platform admins / operators (IT).** Govern the 底座: register and configure hosted micro-apps, manage users and permission scopes, rotate app secrets, review the audit log, and run scheduled jobs. They work in a governance/task context and need density, precision, and controls they can trust.
- **End users.** Discover, launch, and use hosted micro-apps; read notifications in a shared inbox; manage their own profile and settings. They work in a productivity context and need clarity and speed.

Context: authenticated, multi-tenant SaaS, desktop-first (admin and data-heavy surfaces) but responsive. Primary locale is zh-CN. Role determines what's shown, not how the interface works.

## Product Purpose

XGENT.ai Portal is a **general-purpose SaaS platform framework (底座) for hosting micro-apps**. It is the shell every hosted app plugs into: authentication (login / 2FA), a unified app center and launcher, a micro-app host with scoped permissions and token exchange, a shared notification inbox, and a governance layer (users, apps, audit, scheduled tasks).

It is **domain-neutral by design** — the education content in the current prototype (学习助手, 错题本, 课堂, 家长端) is demo flavoring only, not the product's subject. Success means a team can stand up a branded, governed, multi-app portal without rebuilding the shell: the platform stays out of the way so each hosted app reads as itself, while giving operators confident control over access and activity.

Current state: not yet built. A prototype exists (`prototype/`) as the visual and structural reference; the production framework is the work ahead.

## Brand Personality

Match the existing prototype: **precise, engineered, quietly confident.** A developer-platform sensibility (the Linear / Stripe / Vercel family) carried by XGENT's own identity — blue as the workhorse, orange as a sharp accent, cool-tinted neutrals, a geometric sans (Space Grotesk) with mono (JetBrains Mono) for technical data, and a distinctive display voice reserved for hero moments. Voice: clear, exact, technical without jargon for its own sake. The interface should feel like trustworthy infrastructure: calm, dense where the data demands it, never decorative for its own sake.

## Anti-references

- **Dated admin templates** (Bootstrap, AdminLTE, default Ant Design Pro): gray, generic, every-feature-visible, no point of view.
- **Sterile legacy enterprise** (SAP, Oracle, old government portals): heavy, joyless, form-dense.
- **Over-decorated consumer styling**: gradients everywhere, mascots, rounded-everything, gamified or toy-like surfaces.
- Plus the shared AI-slop tells: cream backgrounds, gradient text, an eyebrow above every section, identical card grids.

## Design Principles

1. **The shell disappears; hosted apps shine.** The platform is a frame. It supplies consistent chrome and then gets out of the way so each micro-app's own identity comes through.
2. **Earned familiarity over novelty.** Standard affordances (top bar + side nav, tables, command patterns) done exactly right, so the tool dissolves into the task. No reinvented controls for flavor.
3. **One vocabulary, every role.** Admin and end-user surfaces share the same components, spacing, and state language. Role changes what is shown, not how it behaves.
4. **Governance you can trust at a glance.** Permissions, scopes, tokens, and audit are first-class: legible, precise, and honest about state (success / failure / pending), because operators make security decisions on these screens.
5. **Density with calm.** Dense where data demands it, but quiet: restrained color, accent reserved for action / selection / state, motion only to convey state.

## Accessibility & Inclusion

Target **WCAG 2.1 AA** in both light and dark themes:

- Body text ≥ 4.5:1 contrast, large text ≥ 3:1, placeholders included.
- Full keyboard navigation with a visible focus state on every interactive element (a focus-ring token already exists).
- A `prefers-reduced-motion` alternative for every animation.
- zh-CN is the primary locale; typography and layout must handle CJK well.
- Toward color-blind safety: status colors (success / warning / danger) should pair with an icon or label, never rely on hue alone.
