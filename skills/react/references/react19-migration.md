---
name: react-19-migration
description: Upgrade React 18 source and test files to React 19 — removed APIs, ref as a prop, defaultProps, test-utils, and StrictMode changes.
license: MIT
last_reviewed: 2026-09-26
---

# React 19 migration — removed APIs, source rewrites, and tests

## Contents

- [Ground rule](#ground-rule--an-upgrade-pr-changes-api-surface-only)
- Source files: [root API](#pattern-1--root-api-render--hydrate--unmountcomponentatnode) · [findDOMNode](#pattern-2--finddomnode--a-ref-on-the-dom-element) · [forwardRef](#pattern-3--forwardref--ref-as-a-prop) · [defaultProps](#pattern-4--defaultprops-on-function-components--default-parameters) · [legacy context](#pattern-5--legacy-context--createcontext) · [string refs](#pattern-6--string-refs--createref-or-callback-refs) · [propTypes](#pattern-7--proptypes-are-silently-ignored) · [JSX transform](#pattern-8--new-jsx-transform-and-unused-react-imports) · [TypeScript](#pattern-9--typescript-only-changes)
- Test files: [act](#test-1--import-act-from-react) · [test-utils](#test-2--replace-the-rest-of-react-domtest-utils) · [react-test-renderer](#test-3--react-test-renderer-is-deprecated) · [StrictMode counts](#test-4--strictmode-call-counts-what-actually-changed) · [act warnings](#test-5--not-wrapped-in-act-warnings)
- [Scan checklist](#scan-checklist) · [Decision table](#decision-table--react-19-fix-by-symptom)

## When to apply

- Upgrading `react` / `react-dom` from 18 to 19
- Errors after the upgrade: `ReactDOM.render is not a function`, `findDOMNode is not a function`, a missing `act` export, the "outdated JSX transform" warning
- Code uses `forwardRef`, `defaultProps` on function components, `contextTypes`, string refs, or `react-dom/test-utils`
- Tests fail or change call counts after the upgrade

Most mechanical rewrites have official codemods. Run them first, then fix what's left by hand:

```bash
npx codemod@latest react/19/migration-recipe       # render/hydrate, string refs, act import, useFormState, propTypes
npx types-react-codemod@latest preset-19 ./src     # TypeScript type changes
```

## Ground rule — an upgrade PR changes API surface only

Keep the upgrade diff limited to removed or changed APIs. `createRoot`, `useTransition`, `startTransition`, `useDeferredValue`, `Suspense`, and `lazy` work the same in React 19 — leave that logic untouched. Adopt new APIs ([`use`](data-fetching.md#pattern-5--use-suspense-boundaries-for-non-critical-data), [Actions](actions.md)) in follow-up PRs once the upgrade is green; mixing the two makes regressions impossible to bisect.

```bash
# Files the upgrade touched that also contain concurrent logic — review each diff:
git diff --name-only main... | xargs grep -lE "useTransition|useDeferredValue|startTransition|<Suspense" 2>/dev/null
```

## Source files

### Pattern 1 — Root API: `render` / `hydrate` / `unmountComponentAtNode`

All three are removed from `react-dom`.

**Incorrect (React 18 legacy root):**

```tsx
import ReactDOM from 'react-dom'

ReactDOM.render(<App />, container)
ReactDOM.hydrate(<App />, container)
ReactDOM.unmountComponentAtNode(container)
```

**Correct:**

```tsx
import { createRoot, hydrateRoot } from 'react-dom/client'

const root = createRoot(container)
root.render(<App />)

const ssrRoot = hydrateRoot(container, <App />)

root.unmount()  // needs the root object, not the container
```

`unmount()` needs the root that `createRoot` returned. If the old code only kept the container, store the root where it's created (a module variable, or a `Map` keyed by container for widgets that mount many roots).

### Pattern 2 — `findDOMNode` → a ref on the DOM element

**Incorrect:**

```tsx
class Tooltip extends React.Component {
  componentDidMount() {
    const width = findDOMNode(this).offsetWidth
  }
  render() {
    return <div>{this.props.children}</div>
  }
}
```

**Correct:**

```tsx
class Tooltip extends React.Component {
  nodeRef = React.createRef<HTMLDivElement>()

  componentDidMount() {
    const width = this.nodeRef.current!.offsetWidth
  }
  render() {
    return <div ref={this.nodeRef}>{this.props.children}</div>
  }
}
```

Put the ref on the DOM element the code measures. A ref on a class component points to the instance, not its DOM, so `componentRef.current` is not a drop-in replacement for `findDOMNode(componentRef.current)`.

### Pattern 3 — `forwardRef` → `ref` as a prop

Function components receive `ref` as a regular prop. `forwardRef` still works in React 19, so converting is optional in the upgrade PR — do it when you touch the file.

**Before:**

```tsx
const Input = forwardRef<HTMLInputElement, InputProps>((props, ref) => (
  <input ref={ref} {...props} />
))
```

**After:**

```tsx
function Input({ ref, ...props }: InputProps & { ref?: React.Ref<HTMLInputElement> }) {
  return <input ref={ref} {...props} />
}
```

`useImperativeHandle` is unchanged — only the wrapper goes away (see [Ref rule 3](component-patterns.md#ref-rule-3--use-useimperativehandle-to-limit-exposed-dom-surface)).

Code that inspects children must read `element.props.ref`. `element.ref` is deprecated and warns: "Accessing element.ref is no longer supported. ref is now a regular prop."

### Pattern 4 — `defaultProps` on function components → default parameters

React 19 ignores `defaultProps` on function components. **Class components still support `static defaultProps`** — leave them alone.

**Incorrect (silently ignored in React 19):**

```tsx
function Button({ label, variant }: Props) {
  return <button className={variant}>{label}</button>
}
Button.defaultProps = { label: 'Click', variant: 'primary' }
```

**Correct:**

```tsx
function Button({ label = 'Click', variant = 'primary' }: Props) {
  return <button className={variant}>{label}</button>
}
```

Both mechanisms apply only when the prop is `undefined`; an explicit `null` is passed through either way, so the semantics match.

Watch non-primitive defaults. `defaultProps = { items: [] }` created the array once; `{ items = [] }` creates a new array on every render and breaks `memo` and effect dependencies. Hoist it (see [Pattern 9](rerender-optimization.md#pattern-9--extract-stable-default-values-for-memoized-components)):

```tsx
const NO_ITEMS: Item[] = []

function List({ items = NO_ITEMS }: Props) { /* ... */ }
```

### Pattern 5 — Legacy context → `createContext`

`contextTypes`, `childContextTypes`, and `getChildContext` are removed.

**Incorrect:**

```tsx
class App extends React.Component {
  static childContextTypes = { theme: PropTypes.string }
  getChildContext() { return { theme: 'dark' } }
  render() { return <Toolbar /> }
}

class Button extends React.Component {
  static contextTypes = { theme: PropTypes.string }
  render() { return <button className={this.context.theme} /> }
}
```

**Correct:**

```tsx
const ThemeContext = createContext('light')

function App() {
  return (
    <ThemeContext value="dark">   {/* React 19; older versions use <ThemeContext.Provider> */}
      <Toolbar />
    </ThemeContext>
  )
}

class Button extends React.Component {
  static contextType = ThemeContext   // class consumers: modern contextType is fine
  render() { return <button className={this.context} /> }
}
```

Migrate the provider and every consumer in the same change. The old and new APIs don't see each other, so a half-migrated tree reads `undefined`. Search for all consumers first (`grep -rn "contextTypes" src/`) — they're often in other files.

### Pattern 6 — String refs → `createRef` or callback refs

**Incorrect:**

```tsx
class Search extends React.Component {
  render() {
    return (
      <>
        <input ref="input" />
        <button onClick={() => this.refs.input.focus()}>Focus</button>
      </>
    )
  }
}
```

**Correct:**

```tsx
class Search extends React.Component {
  input = React.createRef<HTMLInputElement>()
  // or a callback ref: input: HTMLInputElement | null = null; <input ref={el => { this.input = el }} />

  render() {
    return (
      <>
        <input ref={this.input} />
        <button onClick={() => this.input.current?.focus()}>Focus</button>
      </>
    )
  }
}
```

### Pattern 7 — `propTypes` are silently ignored

React 19 removed the `propTypes` checks; declarations stay valid but nothing validates at runtime. React recommends migrating to TypeScript (`npx codemod@latest react/prop-types-typescript`). That is a separate change — don't delete `propTypes` in the upgrade PR. If they stay, say so in the code so readers don't assume runtime validation:

```tsx
// NOTE: React 19 no longer runs propTypes validation at runtime.
// PropTypes kept for documentation and IDE tooling only.
Button.propTypes = { label: PropTypes.string }
```

### Pattern 8 — New JSX transform and unused `React` imports

React 19 requires the new JSX transform; without it you get "Your app (or one of its dependencies) is using an outdated JSX transform." With the new transform, `import React from 'react'` is only needed where the file references `React.*`.

```tsx
// Before
import React, { useState } from 'react'

// After (no React.* usage in the file)
import { useState } from 'react'
```

```bash
# Files importing React as a default that never reference React.*:
grep -rlE "^import React[ ,]" src/ | xargs grep -L "React\."
```

### Pattern 9 — TypeScript-only changes

These fail type checking, not at runtime. JavaScript projects need no change.

| Change | Fix |
|---|---|
| `useRef()` requires an argument | `useRef<HTMLDivElement>(null)` or `useRef<number>(undefined)` |
| Ref callbacks may not return a value (the return is now treated as a cleanup) | `ref={el => { instance = el }}`, not `ref={el => (instance = el)}` |
| `ReactElement` props default to `unknown`, not `any` | Type the props, or run `types-react-codemod react-element-default-any-props` |

`preset-19` in `types-react-codemod` handles most of these.

## Test files

Fix in this order — each step unblocks the next:

1. `act` import
2. `Simulate` → `fireEvent`
3. The rest of `react-dom/test-utils`
4. StrictMode call counts — measure, don't guess
5. Remaining "not wrapped in act" warnings
6. The custom render helper — check once per codebase, not per test

### Test 1 — Import `act` from `react`

**Incorrect (removed in React 19):**

```tsx
import { act } from 'react-dom/test-utils'
```

**Correct:**

```tsx
import { act } from 'react'
```

Codemod: `npx codemod@latest react/19/replace-act-import`. When `act` shares an import with other test-utils helpers, split it off and replace the rest (next section).

### Test 2 — Replace the rest of `react-dom/test-utils`

Every other `test-utils` export errors when called in React 19. Use React Testing Library:

| `react-dom/test-utils` | Replacement |
|---|---|
| `act` | `import { act } from 'react'` |
| `Simulate.click(el)`, `.change(el, init)`, `.submit`, `.keyDown` | `fireEvent.click(el)`, `fireEvent.change(el, init)`, … from `@testing-library/react` (same arguments) |
| `renderIntoDocument` | `render` |
| `findRenderedDOMComponentWithTag` / `...WithClass` | `getByRole`, `getByTestId`, or `container.querySelector` |
| `scryRenderedDOMComponentsWithTag` | `getAllByRole` |
| `isElement`, `isCompositeComponent`, `isDOMComponent` | Delete — RTL tests don't inspect component types |

```tsx
// Before
import { act, Simulate } from 'react-dom/test-utils'
Simulate.change(input, { target: { value: 'hello' } })

// After
import { fireEvent } from '@testing-library/react'
fireEvent.change(input, { target: { value: 'hello' } })
```

### Test 3 — `react-test-renderer` is deprecated

In React 19 it logs a deprecation warning and renders concurrently, so snapshots and synchronous assertions may shift. Move those tests to React Testing Library instead of re-recording snapshots.

### Test 4 — StrictMode call counts: what actually changed

StrictMode **still** runs one extra setup → cleanup → setup cycle for every Effect in development (see [Cleanup rule 3](effect-patterns.md#cleanup-rule-3--development-double-fire-is-intentional-fix-cleanup-not-the-guard)). Effect spy counts do not change because of the upgrade — if one did, find out why rather than editing the number.

What changed in React 19 StrictMode:

| Spy on | React 18 StrictMode | React 19 StrictMode |
|---|---|---|
| Component body (render) | 2 per render | 2 per render — unchanged |
| `useEffect` setup | setup, cleanup, setup | Unchanged |
| `useMemo` / `useCallback` factory on mount | Ran in both renders | Second render reuses the first result — count drops |
| Ref callback on initial mount | Once | Invoked, cleaned up, invoked again — count rises |

These counts only apply when the test renders inside StrictMode — an explicit `<StrictMode>` wrapper, a custom render helper, or RTL's `reactStrictMode` option. Check the helper once.

Read the real numbers before editing any assertion:

```bash
npm test -- --watchAll=false --testPathPattern="<file>" 2>&1 | grep -E "Expected|Received"
```

Then match the difference to a row above. An unexplained change is a bug, not a new baseline.

### Test 5 — "not wrapped in act(...)" warnings

React recommends async `act`; the sync form "doesn't work in all cases" and will be removed:

```tsx
await act(async () => {
  fireEvent.click(button)
})
```

RTL's `findBy*` queries and `waitFor` already wrap `act`. Prefer them over manual `act` for anything that resolves asynchronously. Outside RTL, set `globalThis.IS_REACT_ACT_ENVIRONMENT = true` in the test setup.

## Scan checklist

```bash
grep -rnE "ReactDOM\.(render|hydrate)\(|unmountComponentAtNode|findDOMNode" src/   # Pattern 1–2
grep -rn "forwardRef" src/                                                           # Pattern 3 (optional)
grep -rnE "\.defaultProps\s*=" src/                                                  # Pattern 4 — function components only
grep -rnE "contextTypes|childContextTypes|getChildContext" src/                      # Pattern 5
grep -rnE "ref=\"|this\.refs\." src/                                                  # Pattern 6
grep -rnE "\.propTypes\s*=|static propTypes" src/                                    # Pattern 7
grep -rn "useRef()" src/                                                             # Pattern 9 (TypeScript)
grep -rnE "react-dom/test-utils|react-test-renderer" src/                            # Test 1–3
```

## Decision table — React 19 fix by symptom

| Symptom | Fix |
|---|---|
| `ReactDOM.render` / `hydrate` is not a function | `createRoot(container).render()` / `hydrateRoot(container, ui)` |
| `unmountComponentAtNode` is not a function | Keep the root object; call `root.unmount()` |
| `findDOMNode` is not a function | Ref on the DOM element |
| Function component defaults stopped applying | ES6 default parameters; hoist non-primitive defaults |
| `this.context` is `undefined` in a class | Legacy context → `createContext` + `static contextType` |
| `this.refs.x` is `undefined` | `createRef()` or a callback ref |
| PropTypes warnings stopped appearing | Expected — migrate to TypeScript in a separate PR |
| "outdated JSX transform" warning | Enable the automatic JSX runtime in the compiler config |
| `act` import fails | `import { act } from 'react'` |
| `Simulate` / other test-utils throw | React Testing Library equivalents |
| StrictMode spy count changed | Match it to the table in Test 4; effect counts should not change |

## When NOT to apply

- **Class component `defaultProps`:** still supported — rewriting them as constructor logic loses the "only when `undefined`" semantics.
- **Converting every `forwardRef` in the upgrade PR:** it still works; convert when you touch the file.
- **Deleting `propTypes`:** they're harmless; replacing them with types is its own change.
- **Adopting `use`, Actions, or `useOptimistic` during the upgrade:** stabilize first (see the ground rule).

## Common failure modes

| Mistake | Consequence |
|---|---|
| Changing effect-count assertions from 2 to 1 "because React 19 stopped double-invoking" | Wrong premise — tests go green while cleanup bugs go unnoticed |
| Removing `static defaultProps` from class components | Needless rewrite; hand-rolled defaults often mishandle `false` and `0` |
| `{ items = [] }` replacing `defaultProps` in a memoized component | New array every render; `memo` and effect deps break |
| Migrating the legacy context provider but not all consumers | Consumers read `undefined` |
| `componentRef.current` as a `findDOMNode` replacement on a class ref | Gets the instance, not the DOM node |
| Mixing new-API adoption into the upgrade PR | Regressions can't be bisected |
| Sync `act` around async updates | Flaky "not wrapped in act" warnings and missed updates |
