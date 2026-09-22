---
name: react-component-patterns
description: Apply ref, custom hook, and component composition patterns correctly in React.
license: MIT
last_reviewed: 2026-05-02
---

# React component patterns — refs, custom hooks, and composition

## When to apply

- Reading or writing `ref.current` during render causes inconsistent behavior
- A dynamic list needs DOM refs for each item
- A parent holds a ref to a child and accesses too much of its DOM
- A custom hook is named `use*` but calls no hooks internally
- A `useMount` / `useUnmount` wrapper hides missing effect dependencies
- State in a child is duplicated in the parent, causing synchronization bugs
- Multiple intermediate components forward a prop that only the leaf needs

## Ref rule 1 — Never read or write `ref.current` during render

Refs are mutable boxes outside React's render cycle. Mutating them during render makes the output non-deterministic (especially in StrictMode, which renders twice).

**Wrong:**

```tsx
function MyComponent() {
  const ref = useRef(0);
  ref.current++; // mutates during render — different value on re-render
  return <div>{ref.current}</div>; // reading during render — may be stale
}
```

**Correct (read/write only in handlers and effects):**

```tsx
function MyComponent() {
  const ref = useRef(0);

  function handleClick() {
    ref.current++; // OK in event handler
  }

  useEffect(() => {
    ref.current = computedValue; // OK in effect
  }, [computedValue]);

  return <button onClick={handleClick}>Click</button>;
}
```

## Ref rule 2 — Use ref callbacks for dynamic lists

`useRef` inside a loop violates Rules of Hooks. Use a `Map` ref with a callback instead.

**Wrong:**

```tsx
{items.map((item) => {
  const ref = useRef(null); // Rules of Hooks violation
  return <li key={item.id} ref={ref} />;
})}
```

**Correct:**

```tsx
const itemsRef = useRef(new Map());

{items.map((item) => (
  <li
    key={item.id}
    ref={(node) => {
      if (node) {
        itemsRef.current.set(item.id, node);
      } else {
        itemsRef.current.delete(item.id);
      }
    }}
  />
))}

// Access a specific item DOM node:
// itemsRef.current.get(targetId)?.scrollIntoView()
```

## Ref rule 3 — Use `useImperativeHandle` to limit exposed DOM surface

When a parent holds a ref to a child, expose only the methods you intend — not the full DOM node.

```tsx
function MyInput({ ref }: { ref: React.Ref<{ focus(): void }> }) {
  const inputRef = useRef<HTMLInputElement>(null);

  useImperativeHandle(ref, () => ({
    focus() {
      inputRef.current?.focus();
    },
    // Parent can ONLY call focus(). It cannot access .value, .style, etc.
  }));

  return <input ref={inputRef} />;
}
```

Without `useImperativeHandle`, the parent can mutate any property on the DOM node, breaking encapsulation.

## Custom hook rule 1 — Hooks share logic, not state

Each call to a custom hook creates independent state. Two components calling `useOnlineStatus()` do not share a subscription — they each get their own.

```tsx
function StatusBar() {
  const isOnline = useOnlineStatus(); // independent subscription
}

function SaveButton() {
  const isOnline = useOnlineStatus(); // independent subscription
}
```

To share state between components, lift it up or use context — not a custom hook.

## Custom hook rule 2 — Name `use*` only if the function calls hooks

The `use` prefix tells the linter (and other developers) that Rules of Hooks apply. Misusing it disables lint checks.

**Wrong:**

```tsx
function useSorted(items: Item[]) {
  return items.slice().sort(); // no hooks called — misleading name
}
```

**Correct:**

```tsx
function getSorted(items: Item[]) {
  return items.slice().sort(); // plain function
}

function useFilteredItems(category: string) {
  const items = useContext(ItemsContext); // actually uses a hook
  return useMemo(() => items.filter(i => i.category === category), [items, category]);
}
```

## Custom hook rule 3 — Avoid lifecycle wrapper hooks

`useMount` and `useUnmount` wrappers pass a function to an empty-dep effect. The linter cannot see inside the wrapper to catch missing dependencies.

**Wrong:**

```tsx
function useMount(fn: () => void) {
  useEffect(() => {
    fn();
  }, []); // fn missing from deps — linter can't warn about stale closures inside fn
}
```

**Correct (use `useEffect` directly):**

```tsx
useEffect(() => {
  doSomethingWith(value); // linter sees `value` and warns if it's missing
}, [value]);
```

The only valid use for lifecycle wrappers is logging or observability, where you explicitly don't want reactive re-runs.

## Component rule 1 — Prefer controlled components when the parent cares about the value

| Style | Who owns state | When to use |
|---|---|---|
| Uncontrolled | Component itself | Isolated input with no external sync |
| Controlled | Parent via props | Testing, form validation, parent-level reset |

```tsx
// Uncontrolled — component owns its query
function SearchInput() {
  const [query, setQuery] = useState('');
  return <input value={query} onChange={e => setQuery(e.target.value)} />;
}

// Controlled — parent owns the query; easier to test and compose
function SearchInput({ query, onQueryChange }: Props) {
  return <input value={query} onChange={e => onQueryChange(e.target.value)} />;
}
```

## Component rule 2 — Prefer composition over prop drilling

Prop drilling (passing a value through N components that don't use it) is the wrong default. Try composition first; reach for context only when composition isn't practical.

**Wrong (every layer receives and forwards `user`):**

```tsx
<App user={user}>
  <Layout user={user}>
    <Header user={user}>
      <Avatar user={user} />
    </Header>
  </Layout>
</App>
```

**Correct (pass the rendered node; intermediate components don't see `user`):**

```tsx
<App>
  <Layout>
    <Header avatar={<Avatar user={user} />} />
  </Layout>
</App>
```

**When composition isn't enough (global, read-only state like auth or locale):**

```tsx
<UserContext.Provider value={user}>
  <App />
</UserContext.Provider>
```

## Component rule 3 — Use `flushSync` to read the DOM after a synchronous state update

React batches state updates by default. When you must read updated layout immediately (e.g., scroll to a new item), force synchronous flush.

```tsx
import { flushSync } from 'react-dom';

function handleAdd() {
  flushSync(() => {
    setTodos([...todos, newTodo]);
  });
  // DOM is now updated — safe to measure or scroll
  listRef.current?.lastElementChild?.scrollIntoView();
}
```

Do NOT use `flushSync` as a general pattern. It disables batching and hurts performance. Use it only when you need to read layout immediately after a state update.

## Decision table — which pattern to apply

| Situation | Pattern |
|---|---|
| Reading or mutating ref during render | Move to event handler or effect |
| `useRef` called inside a map | Ref callback with `Map` |
| Parent ref exposes full DOM node | `useImperativeHandle` |
| Two components need shared subscription result | Lift state or use context |
| Utility function named `useFoo` with no hooks | Rename to `getFoo` |
| `useMount(fn)` hides dependency | Replace with `useEffect(() => fn(), [dep])` |
| Child state duplicated in parent | Lift state; make child controlled |
| Prop passed through 3+ components that don't use it | Composition (pass the element) or context |
| Must scroll to DOM node after state update | `flushSync` |

## When NOT to apply

- **`useImperativeHandle`:** If the parent only needs to focus/scroll the child and a regular forwarded ref would work, skip the extra indirection.
- **Controlled components everywhere:** UI-only widgets (date pickers, color swatches) with no external state needs work fine as uncontrolled.
- **`flushSync`:** Never in render paths, loops, or event handlers where batching is beneficial. The cost is always a synchronous re-render.

## Common failure modes

| Mistake | Consequence |
|---|---|
| Writing to `ref.current` during render | Non-deterministic output; StrictMode double-render shows inconsistent values |
| `useRef(null)` inside `.map()` | Rules of Hooks violation; random crashes |
| Forwarding a ref without `useImperativeHandle` | Parent mutates arbitrary DOM properties, breaking encapsulation |
| `useMount(fn)` wrapping stale closure | Silent bug: `fn` always captures values from first render |
| `use` prefix on a function with no hooks | Linter may flag valid call sites or skip invalid ones |
| Prop drilling vs. composition | Refactoring cost multiplies with every new intermediate layer |
| `flushSync` inside another `flushSync` | Throws; React does not support nested synchronous flushes |
