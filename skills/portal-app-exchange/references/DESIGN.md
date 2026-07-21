---
name: XGENT.ai Portal
description: A precise, quietly confident shell for a general-purpose micro-app platform (底座).
colors:
  brand-blue: "#0063D3"
  brand-blue-hover: "#0054B3"
  brand-blue-press: "#00408A"
  brand-orange: "#FF6B02"
  brand-orange-hover: "#DB5A00"
  ink: "#0B1220"
  ink-2: "#1A2233"
  ink-3: "#2A3447"
  paper: "#FAFAFC"
  white: "#FFFFFF"
  slate-50: "#F2F4F9"
  slate-100: "#E7EBF2"
  slate-200: "#D5DBE6"
  slate-300: "#B6BECF"
  slate-500: "#6E7A94"
  slate-600: "#54607A"
  slate-700: "#3E4A60"
  success-500: "#1F9D55"
  warning-500: "#D89400"
  danger-500: "#D6293E"
typography:
  display:
    fontFamily: "Iceland, ui-monospace, monospace"
    fontSize: "96px"
    fontWeight: 400
    lineHeight: 1.15
    letterSpacing: "-0.02em"
  headline:
    fontFamily: "Space Grotesk, ui-sans-serif, system-ui, sans-serif"
    fontSize: "48px"
    fontWeight: 600
    lineHeight: 1.15
    letterSpacing: "-0.02em"
  title:
    fontFamily: "Space Grotesk, ui-sans-serif, system-ui, sans-serif"
    fontSize: "32px"
    fontWeight: 600
    lineHeight: 1.3
    letterSpacing: "-0.02em"
  body:
    fontFamily: "Space Grotesk, ui-sans-serif, system-ui, sans-serif"
    fontSize: "16px"
    fontWeight: 400
    lineHeight: 1.5
    letterSpacing: "0"
  label:
    fontFamily: "Space Grotesk, ui-sans-serif, system-ui, sans-serif"
    fontSize: "14px"
    fontWeight: 500
    lineHeight: 1.3
    letterSpacing: "0"
  mono:
    fontFamily: "JetBrains Mono, ui-monospace, SF Mono, Menlo, monospace"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.5
    letterSpacing: "0"
rounded:
  sm: "4px"
  md: "8px"
  lg: "12px"
  xl: "16px"
  pill: "999px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "16px"
  lg: "24px"
  xl: "32px"
  "2xl": "48px"
  "3xl": "64px"
components:
  button-primary:
    backgroundColor: "{colors.brand-blue}"
    textColor: "{colors.white}"
    rounded: "{rounded.md}"
    height: "36px"
    padding: "0 14px"
  button-primary-hover:
    backgroundColor: "{colors.brand-blue-hover}"
    textColor: "{colors.white}"
    rounded: "{rounded.md}"
    height: "36px"
    padding: "0 14px"
  button-secondary:
    backgroundColor: "{colors.white}"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    height: "36px"
    padding: "0 14px"
  button-ghost:
    backgroundColor: "transparent"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    height: "36px"
    padding: "0 14px"
  button-accent:
    backgroundColor: "{colors.brand-orange}"
    textColor: "{colors.white}"
    rounded: "{rounded.md}"
    height: "36px"
    padding: "0 14px"
  button-danger:
    backgroundColor: "{colors.danger-500}"
    textColor: "{colors.white}"
    rounded: "{rounded.md}"
    height: "36px"
    padding: "0 14px"
  input:
    backgroundColor: "{colors.white}"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    height: "40px"
    padding: "0 12px"
  badge:
    backgroundColor: "{colors.slate-100}"
    textColor: "{colors.slate-700}"
    rounded: "{rounded.pill}"
    padding: "3px 9px"
  chip:
    backgroundColor: "{colors.slate-50}"
    textColor: "{colors.ink}"
    rounded: "{rounded.pill}"
    padding: "4px 6px 4px 10px"
  card:
    backgroundColor: "{colors.white}"
    textColor: "{colors.ink}"
    rounded: "{rounded.lg}"
    padding: "20px"
  nav-item:
    backgroundColor: "transparent"
    textColor: "{colors.slate-600}"
    rounded: "{rounded.md}"
    height: "38px"
    padding: "8px 10px"
  nav-item-active:
    backgroundColor: "transparent"
    textColor: "{colors.brand-blue}"
    rounded: "{rounded.md}"
    height: "38px"
    padding: "8px 10px"
---

# Design System: XGENT.ai Portal

## 1. Overview

**Creative North Star: "The Quiet Substrate"**

XGENT.ai Portal is the floor every hosted micro-app stands on, not a stage that competes with them. The shell supplies precise, trustworthy chrome (a top bar, a side nav, dialogs, tables, governance views) and then recedes, so each hosted app carries its own color and identity. The aesthetic is engineered and quietly confident: cool-tinted neutrals, a single workhorse blue, an orange accent used sparingly, and type that stays exact at every size. It belongs to the developer-platform family (the Linear / Stripe / Vercel sensibility) carried by XGENT's own identity rather than copying anyone's surface.

Density is a feature here, not a flaw. Operators read audit logs, manage scopes and tokens, and schedule jobs; end-users scan a launcher and an inbox. The system serves both by keeping one component vocabulary across every screen and changing only what is shown, never how it behaves. Color is restrained on purpose: the accent earns attention precisely because it is rare, and status is always carried by an icon or label, never hue alone.

This system explicitly rejects the dated admin template (gray, generic, every-feature-visible Bootstrap / AdminLTE / default Ant Design Pro), sterile legacy enterprise (heavy, joyless, form-dense SAP / Oracle), over-decorated consumer styling (gradients everywhere, mascots, rounded-everything), and the AI-slop tells (cream backgrounds, gradient text, an eyebrow above every section, identical card grids).

**Key Characteristics:**
- Cool-tinted slate neutrals over a near-white `paper` (#FAFAFC), with a fully realized dark theme on `ink` (#0B1220).
- One saturated brand blue (#0063D3) as the workhorse; orange (#FF6B02) as a sparing accent for unread / new-since states.
- A geometric sans (Space Grotesk) for nearly everything; mono (JetBrains Mono) for IDs, tokens, cron, and code; a display face (Iceland) reserved for rare hero moments.
- Restrained components: 8px radii on controls, hairline borders, hover/focus felt more than seen.
- Light and dark are first-class, driven by `data-theme` on the root; both must clear WCAG 2.1 AA.

## 2. Colors

A cool, engineered palette: slate neutrals harmonized toward the brand blue, one decisive blue, and a single warm accent held in reserve.

### Primary
- **XGENT Blue** (#0063D3): The workhorse. Primary buttons, the active nav state, current selection, focus rings, links, and info status. Hover deepens to **#0054B3**, press to **#00408A**. Selected/hover backgrounds use it at 8% (`rgba(0,99,211,0.08)`) and 16% tints, never at full saturation on a resting surface.

### Secondary
- **XGENT Orange** (#FF6B02): The accent, used sparingly for signal: unread badges, "new since," the notification dot, and the occasional accent button. Hover deepens to **#DB5A00**; on dark it warms slightly to #FF7A1A. If orange is carrying more than a small fraction of a screen, something is wrong.

### Neutral
- **Ink** (#0B1220): Primary text in light mode; the body background in dark mode. The whole shadow system is tinted with it (`rgba(11,18,32,...)`).
- **Slate 600 / 500** (#54607A / #6E7A94): Secondary and tertiary text, meta, icon defaults.
- **Slate 300 / 200** (#B6BECF / #D5DBE6): Strong borders (inputs, control outlines) and hairline borders (dividers, card edges).
- **Slate 100 / 50** (#E7EBF2 / #F2F4F9): Sunken surfaces and the second panel layer (search field, segmented track, skeletons).
- **Paper / White** (#FAFAFC / #FFFFFF): App background and raised surface in light mode.
- **Ink 2 / Ink 3** (#1A2233 / #2A3447): Raised surface and second surface layer in dark mode.

### Status
- **Success** (#1F9D55), **Warning** (#D89400), **Danger** (#D6293E): Each pairs with a 50-tint background (#E7F6EE / #FBF3DD / #FCE6E9) for badges, and always travels with an icon or text label. Info reuses the brand blue.

### Named Rules
**The One Accent Rule.** Orange is signal, not decoration. It marks the unread, the new, the single most important action on a surface, and nothing else. Its rarity is the entire point; the moment it repeats, it stops meaning anything.

**The Tint, Not Fill Rule.** Selected and hovered surfaces use the brand blue at 8–16% opacity over the neutral, never the solid blue. Solid blue is reserved for the thing you are meant to click or the state you are meant to read.

**The Borrowed-Color Rule.** Each hosted micro-app owns an identity color (its glyph, its accents). The shell stays neutral so those colors read; the platform never competes with the app it hosts.

## 3. Typography

**Display Font:** Iceland (with `ui-monospace, monospace` fallback)
**Body / UI Font:** Space Grotesk (with `ui-sans-serif, system-ui, -apple-system, sans-serif` fallback)
**Mono Font:** JetBrains Mono (with `ui-monospace, SF Mono, Menlo, Consolas, monospace` fallback)

**Character:** Space Grotesk carries the entire interface (headings, buttons, labels, body, data) with enough geometric personality to feel engineered without ever shouting. JetBrains Mono handles anything machine-shaped: IDs, scopes, tokens, cron expressions, IPs, code. Iceland is a deliberate, rare display voice for hero moments and the wordmark register; it never appears in UI labels, buttons, or data.

### Hierarchy
- **Display** (Iceland, 96px / 64px, 400, line-height 1.15, uppercase, tracking -0.02em): Hero moments only (login splash, marketing-adjacent surfaces). Never inside the app shell.
- **Headline / h1** (Space Grotesk, 48px, 600, line-height 1.15, tracking -0.02em): Page titles. Use `text-wrap: balance`.
- **Title / h2–h3** (Space Grotesk, 32px → 20px, 600, line-height 1.3): Section and card headings.
- **Body** (Space Grotesk, 16px, 400, line-height 1.5): Default reading text. Cap prose at 65–75ch; dense tables and data may run wider.
- **Label** (Space Grotesk, 14px, 500, line-height 1.3): Form labels, pill text, nav items, button text (600 in buttons).
- **Eyebrow / overline** (Space Grotesk, 12px, 600, uppercase, tracking 0.08em, brand blue): Reserved for things like group dividers in the nav and dropdown section headers. Not an automatic kicker above every section.
- **Mono** (JetBrains Mono, ~14px, 400): IDs, tokens, scopes, cron, IP, keyboard hints (`⌘K`), inline code.

### Named Rules
**The Iceland-Is-Rare Rule.** The display face is a guest, not a workhorse. If it shows up on a button, a label, a table header, or more than once on a screen, replace it with Space Grotesk.

**The Mono-Means-Machine Rule.** Monospace signals "this is a literal machine value" (a token, a scope, a cron string, an IP). Don't use it for emphasis or flavor; don't set human prose in it.

## 4. Elevation

The system is mostly flat, with a cool-tinted shadow ramp reserved for things that genuinely float. Depth is structural, not decorative: panels and cards sit flat against their surface and rely on hairline borders and the second neutral layer for separation; shadow appears only when an element leaves the plane (popovers, modals, drawers, the lifted segmented thumb). Every shadow is tinted with ink (`rgba(11,18,32,...)`) in light mode and pure black in dark mode, so elevation stays in the same cool family as the rest of the palette.

### Shadow Vocabulary
- **Shadow 1** (`0 1px 2px rgba(11,18,32,.04), 0 1px 1px rgba(11,18,32,.06)`): Contact shadow for small raised elements (app glyphs, the segmented active thumb, the toggle knob).
- **Shadow 2** (`0 4px 12px rgba(11,18,32,.06), 0 2px 4px rgba(11,18,32,.04)`): Resting cards that need to lift slightly off the page.
- **Shadow 3** (`0 12px 28px rgba(11,18,32,.10), 0 4px 8px rgba(11,18,32,.06)`): Popovers and anchored dropdowns.
- **Shadow 4** (`0 24px 56px rgba(11,18,32,.18), 0 8px 16px rgba(11,18,32,.10)`): Modals and drawers, the top of the stack.
- **Focus ring** (`0 0 0 2px var(--bg), 0 0 0 4px var(--brand-blue)`): The single focus treatment for keyboard navigation; inputs additionally show a 3px brand-blue 16% glow.

### Named Rules
**The Flat-At-Rest Rule.** Surfaces are flat by default. A shadow is a response to leaving the plane (open a popover, raise a modal, lift the active segment), never a default decoration on a static card. If a resting card has Shadow 3 on it, it is wrong.

## 5. Components

Every interactive component carries default, hover, focus, active, disabled states, plus loading (Skeleton) and empty/error (StateBlock) where content can be absent. The feel is precise and restrained: crisp 8px radii, hairline borders, transitions you feel more than see (120ms ease-out).

### Buttons
- **Shape:** 8px radius (`rounded.md`), four heights (xs 26 / sm 30 / md 36 / lg 44px), weight 600, single-line.
- **Primary:** Solid brand blue, white text. Hover → #0054B3. The default affirmative action.
- **Secondary:** White surface, ink text, strong slate border (#B6BECF). Hover → slate-50.
- **Ghost:** Transparent, ink text, no border. Hover → slate-50. For low-emphasis actions in dense toolbars.
- **Accent:** Solid orange, white text. Reserved for a single high-signal action; not a second primary.
- **Danger:** Solid #D6293E, white text. Destructive actions only.
- **Disabled:** 40% opacity, `not-allowed`. **Hover / focus:** background shift + the global focus ring; transition `background 120ms`, `transform 80ms`.
- **Icon button:** 36px square, 8px radius, slate-2 icon; active state uses the brand-blue 8% tint with blue icon; supports a count badge (orange).

### Chips & Badges
- **Badge:** Pill (`rounded.pill`), 12px / 600, tone backgrounds at the 50-tint with a darker same-hue text (e.g. blue bg #E6F0FB / text #003E85). Optional leading dot. Tones: blue, orange, success, warning, danger, neutral, solid.
- **Chip:** Pill, 13px / 500, slate-50 (or brand-blue 8% for the blue tone) with a hairline border and an optional remove affordance. For multi-value inputs and filters.

### Cards / Containers
- **Corner Style:** 12px (`rounded.lg`); modals/drawers go to 16px (`rounded.xl`).
- **Background:** White (light) / ink-2 (dark).
- **Shadow Strategy:** Flat at rest (see Elevation). Use a hairline `--border` (#D5DBE6) for separation; reserve Shadow 2 for cards that genuinely lift.
- **Border:** 1px slate-200 hairline.
- **Internal Padding:** 20–24px (`spacing.lg`).
- **Nesting:** Never put a card inside a card.

### Inputs / Fields
- **Style:** 40px height, white surface, 1px strong-slate border (#B6BECF), 8px radius, 14px text, with optional leading icon and a `*` required marker.
- **Focus:** Border shifts to brand blue + a 3px brand-blue 16% glow (`0 0 0 3px rgba(0,99,211,.16)`).
- **Error:** Border and helper text turn danger (#D6293E); helper line carries the message.
- **Controls:** Toggle (40×22 pill, blue when on), Radio (blue ring + dot), Checkbox (blue fill + white check, 5px radius), Segmented (slate-2 track, white lifted thumb with Shadow 1).

### Navigation
- **Top bar:** 56px, white/ink-2 surface, hairline bottom border. Holds the menu toggle, wordmark, tenant switcher, a centered search field (slate-2 fill with a `⌘K` mono hint), and the right cluster (app launcher, notification bell with orange count, user menu).
- **Side nav:** 240px expanded / 64px collapsed (width transitions 200ms), grouped with 10.5px uppercase tracked group labels. **Active item:** brand-blue 8% tint, blue text/icon at weight 600, plus a 3px rounded brand-blue rail pinned to the left edge of that item. **Hover:** slate-50. Collapsed mode centers icons and shows labels as tooltips.
- **Popover:** Anchored dropdown, 12px radius, Shadow 3, click-outside + Escape to close, `xpop` entrance (160ms). Used for tenant switch, app launcher, notifications, user menu.

### Signature: App Glyph
A rounded square (12px radius) filled with the hosting app's own identity color, a white line icon centered, Shadow 1. This is how a hosted micro-app announces itself inside the neutral shell, in the launcher, the nav, notifications, and the app center. It is the one place per row where a non-brand color is not only allowed but expected.

## 6. Do's and Don'ts

### Do:
- **Do** keep the brand blue as the single workhorse: primary actions, active nav, selection, focus, links.
- **Do** carry selection and hover as the brand-blue 8–16% tint over the neutral, not solid blue.
- **Do** pair every status color with an icon or label (color-blind safety; never hue alone).
- **Do** let each hosted app's identity color live in its App Glyph, and keep the surrounding shell neutral so it reads.
- **Do** keep one component vocabulary across admin and end-user screens; role changes what's shown, not how it works.
- **Do** use mono (JetBrains Mono) for literal machine values (tokens, scopes, cron, IPs) and Skeletons for loading, StateBlocks for empty/error.
- **Do** ship a `prefers-reduced-motion` alternative for every animation (`xfade`, `xrise`, `xslide`, `xpop`, `xshimmer`); transitions sit at 120–280ms ease-out.
- **Do** keep both light and dark themes at WCAG 2.1 AA: body ≥ 4.5:1, large text ≥ 3:1, placeholders included.

### Don't:
- **Don't** build the dated admin template look: gray, generic, every-feature-visible Bootstrap / AdminLTE / default Ant Design Pro chrome.
- **Don't** drift into sterile legacy enterprise (heavy, joyless, form-dense SAP / Oracle / old gov portal density without hierarchy).
- **Don't** over-decorate like a consumer toy: gradients everywhere, mascots, rounded-everything, gamified surfaces.
- **Don't** use the AI-slop tells: cream/sand backgrounds, gradient text (`background-clip: text`), an uppercase tracked eyebrow above every section, or identical icon-heading-text card grids.
- **Don't** put a colored side-stripe `border-left` on cards, list items, callouts, or alerts. (The 3px rail on the *active nav item* is the one sanctioned exception, and only there.)
- **Don't** set Iceland anywhere in UI labels, buttons, or data; don't set human prose in mono.
- **Don't** put shadows on resting cards, or nest a card inside a card.
- **Don't** reach for a modal first; exhaust inline, drawer, and progressive alternatives before interrupting the user.
- **Don't** let orange become decoration; if it repeats across a screen, it has stopped being signal.
