# Operate mode depth (and Read notes)

When design SERVES the product: app UIs, admin dashboards, settings panels, data tables, tools, authenticated surfaces, anything where the user is in a task. The essentials live in SKILL.md's modes and [craft-floor.md](craft-floor.md); this file is extended depth, written for Operate surfaces. Read surfaces (docs, guides, long-form) take SKILL.md's Read mode plus this file's typography and consistency rules; their prose measure and navigation matter more than component density.

## The product slop test

Familiarity is often a feature here. The test is whether a category-fluent user can trust the interface immediately or must pause at every subtly-off component.

Product UI's failure mode isn't flatness, it's strangeness without purpose: over-decorated buttons, mismatched form controls, gratuitous motion, display fonts where labels should be, invented affordances for standard tasks. The bar is earned familiarity. The tool should disappear into the task.

## Typography

- **One family is often right.** Product UIs don't need display/body pairing. A well-tuned sans carries headings, buttons, labels, body, data.
- **Fixed rem scale, not fluid.** Clamp-sized headings don't serve product UI. Users view at consistent DPI, and a fluid h1 that shrinks in a sidebar looks worse, not better.
- **Tighter scale ratio.** 1.125–1.2 between steps is typical. More type elements here than on brand surfaces; exaggerated contrast creates noise.
- **Line length still applies for prose** (65–75ch). Data and compact UI can run denser; tables at 120ch+ are fine.

## Color

Product defaults to Restrained. A single surface can earn Committed (a dashboard where one category color carries a report, an onboarding flow with a drenched welcome screen), but Restrained is the floor.

- State-rich semantic vocabulary: hover, focus, active, disabled, selected, loading, error, warning, success, info. Standardize these.
- Accent color used for primary actions, current selection, and state indicators only, not decoration.
- A second neutral layer for sidebars, toolbars, and panels (slightly cooler or warmer than the content surface).

## Layout

- Responsive behavior is structural (collapse sidebar, responsive table, breakpoint-driven columns), not fluid typography.

## Components

Every interactive component has: default, hover, focus, active, disabled, loading, error. Don't ship with half of these.

- Skeleton states for loading, not spinners in the middle of content.
- Empty states that teach the interface, not "nothing here."
- Consistent affordances across the surface. Same button shape. Same form-control vocabulary. Same icon style.
- Overlays escape their container. An absolutely positioned dropdown inside an `overflow: hidden` or `overflow: auto` ancestor gets clipped; reach for `<dialog>`, the popover API, `position: fixed`, or a portal.

### Controls and forms

The seven states above still apply. These are the mechanics that make a form feel engineered rather than assembled.

- **States never move the geometry.** Border widths stay equal across default, hover, focus, active, disabled, loading, and error. Draw focus with an outline or `box-shadow`, and reserve the focus ring's space at rest with a transparent outline so nothing shifts when focus lands.
- **Focus appears immediately.** Never fade a focus indicator in. Don't remove the outline unless `:focus-visible` supplies another visible marker — a ring, a border, or a background change.
- **Fields and the buttons beside them share one control height.** An input and its adjacent button in the same row match; neither towers over nor crouches under the other.
- **The label lives outside the field; a placeholder is not a label.** Help text and error text share one reserved line, so showing a validation message doesn't move the form.
- **Errors carry text and `aria-invalid`, not just color.** Disabled uses the native `disabled` attribute or `aria-disabled`, not merely reduced opacity.
- **Hover styles sit inside `@media (hover: hover)`, and no function depends on hover alone.** adapt.md owns input-method detection and the hover split; for form controls the extra rule is that `:focus-visible` must offer the same affordance the hover state gives a mouse user.
- **Numbers line up.** Numeric columns and counters that change get `font-variant-numeric: tabular-nums`.
- **Finish in place when the result is visible; keep toasts for failures and for results the user cannot see.** Prefer undo over a confirmation dialog for reversible actions. Silence does not hide the undo entry: place it where the risk deserves — toast, inline, or menu inside the surface — and never let the quiet default make undo unfindable.
- **Button, tab, and navigation labels stay one readable unit at every supported width.** Reflow the row or move the control so the label fits; don't let the label itself wrap onto a second line and don't force it back onto one with `nowrap` or truncation — the first trips the wrapped-control-label check, the second the built-in text-overflow check. When the project or a localization requirement makes a longer label unavoidable, follow that rule and give the control the room it needs.
- **Inspect hover, focus-visible, and active with forced pseudo-classes** in DevTools or CDP. Don't add preview-only classes to product styles.

## Motion

- 150–250 ms on most transitions. Users are in flow; don't make them wait for choreography.
- Motion conveys state, not decoration. State change, feedback, loading, reveal: nothing else.
- No orchestrated page-load sequences. Product loads into a task; users don't want to watch it load.

## Product constraints

- Decorative motion that doesn't convey state.
- Inconsistent component vocabulary across screens. If the "save" button looks different in two places, one is wrong.
- Display fonts in UI labels, buttons, data.
- Reinventing standard affordances for flavor (custom scrollbars, weird form controls, non-standard modals).
- Heavy color or full-saturation accents on inactive states.
- Modal as first thought. Modals are usually laziness. Exhaust inline / progressive alternatives first.

## Product permissions

Product can afford things brand surfaces can't.

- System fonts and familiar sans defaults.
- Standard navigation patterns: top bar + side nav, breadcrumbs, tabs, command palettes.
- Density. Tables with many rows, panels with many labels, dense information when users need it.
- Consistency over surprise. The same visual vocabulary screen to screen is a virtue; delight is saved for moments, not pages.
