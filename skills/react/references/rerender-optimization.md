---
name: react-rerender-optimization
description: Reduce unnecessary React component re-renders and optimize hook usage in React applications.
license: MIT
last_reviewed: 2026-05-02
---

# React re-render optimization — memoization, state, and hooks

## When to apply

- Parent state changes trigger child re-renders the child doesn't need
- `useEffect` runs on every render due to object/array dependencies
- `useMemo` wrapping simple boolean expressions
- State updates based on previous state cause stale closures
- Expensive computations run on every render even when inputs haven't changed

## Pattern 1 — Extract to memoized components

Move expensive work into `memo()`-wrapped child components to enable early returns before computation.

**Incorrect (computes avatar even when loading):**

```tsx
function Profile({ user, loading }: Props) {
  const avatar = useMemo(() => {
    const id = computeAvatarId(user)
    return <Avatar id={id} />
  }, [user])

  if (loading) return <Skeleton />
  return <div>{avatar}</div>
}
```

**Correct (skips computation when loading):**

```tsx
const UserAvatar = memo(function UserAvatar({ user }: { user: User }) {
  const id = useMemo(() => computeAvatarId(user), [user])
  return <Avatar id={id} />
})

function Profile({ user, loading }: Props) {
  if (loading) return <Skeleton />
  return (
    <div>
      <UserAvatar user={user} />
    </div>
  )
}
```

> If React Compiler is enabled, manual memoization is unnecessary. This article assumes manual optimization.

## Pattern 2 — Don't define components inside components

A component defined inside another creates a new type on every parent render. React remounts it, destroying all state and DOM.

**Incorrect (remounts on every render):**

```tsx
function UserProfile({ user, theme }) {
  const Avatar = () => (
    <img src={user.avatarUrl} className={theme === 'dark' ? 'dark' : 'light'} />
  )
  return <div><Avatar /></div>
}
```

**Correct (pass props instead):**

```tsx
function Avatar({ src, theme }: { src: string; theme: string }) {
  return <img src={src} className={theme === 'dark' ? 'dark' : 'light'} />
}

function UserProfile({ user, theme }) {
  return <div><Avatar src={user.avatarUrl} theme={theme} /></div>
}
```

Symptoms: input fields lose focus on keystroke, animations restart, `useEffect` cleanup/setup runs on every parent render.

## Pattern 3 — Calculate derived state during render

If a value can be computed from props/state, derive it during render. Don't store it in state or update it in an effect.

**Incorrect (redundant state + effect):**

```tsx
function Form() {
  const [first, setFirst] = useState('')
  const [last, setLast] = useState('')
  const [full, setFull] = useState('')

  useEffect(() => {
    setFull(first + ' ' + last)
  }, [first, last])

  return <p>{full}</p>
}
```

**Correct (derive during render):**

```tsx
function Form() {
  const [first, setFirst] = useState('')
  const [last, setLast] = useState('')
  const full = first + ' ' + last

  return <p>{full}</p>
}
```

## Pattern 4 — Use functional setState for updates based on previous state

Prevents stale closures and eliminates unnecessary dependencies.

**Incorrect (stale closure risk):**

```tsx
const removeItem = useCallback((id: string) => {
  setItems(items.filter(i => i.id !== id))  // Missing items dependency
}, [])  // Bug: always references initial items
```

**Correct (always uses latest state):**

```tsx
const removeItem = useCallback((id: string) => {
  setItems(curr => curr.filter(i => i.id !== id))
}, [])  // No dependencies needed
```

Use functional updates whenever the new state depends on the previous value.

## Pattern 5 — Use lazy state initialization for expensive defaults

Without the function form, the initializer runs on every render.

**Incorrect (runs on every render):**

```tsx
const [index] = useState(buildSearchIndex(items))
const [settings] = useState(JSON.parse(localStorage.getItem('settings') || '{}'))
```

**Correct (runs once):**

```tsx
const [index] = useState(() => buildSearchIndex(items))
const [settings] = useState(() => {
  const stored = localStorage.getItem('settings')
  return stored ? JSON.parse(stored) : {}
})
```

For simple primitives (`useState(0)`), the function form is unnecessary.

## Pattern 6 — Don't wrap simple expressions in useMemo

`useMemo` has overhead. For simple expressions with primitive results, compute directly.

**Incorrect:**

```tsx
const isLoading = useMemo(() => {
  return user.isLoading || notifications.isLoading
}, [user.isLoading, notifications.isLoading])
```

**Correct:**

```tsx
const isLoading = user.isLoading || notifications.isLoading
```

**To find out whether a calculation is worth memoizing, measure it:**

```tsx
console.time('filter')
const visibleTodos = getFilteredTodos(todos, filter)
console.timeEnd('filter')
```

Above ~1ms it's worth memoizing. Measure with CPU throttling on, since development machines are faster than users' devices.

## Pattern 7 — Narrow effect dependencies

Use primitive dependencies instead of objects. Compute derived booleans outside the effect.

**Incorrect (re-runs on any user field change):**

```tsx
useEffect(() => {
  console.log(user.id)
}, [user])
```

**Correct (re-runs only when id changes):**

```tsx
useEffect(() => {
  console.log(user.id)
}, [user.id])
```

**For continuous values, derive a boolean first:**

```tsx
// Incorrect: runs on width=767, 766, 765...
useEffect(() => {
  if (width < 768) enableMobileMode()
}, [width])

// Correct: runs only on boolean transition
const isMobile = width < 768
useEffect(() => {
  if (isMobile) enableMobileMode()
}, [isMobile])
```

## Pattern 8 — Split combined hook computations

A combined `useMemo` or `useEffect` reruns all tasks when any dependency changes.

**Incorrect (changing `sortOrder` recomputes filtering):**

```tsx
const sorted = useMemo(() => {
  const filtered = products.filter(p => p.category === category)
  return filtered.toSorted((a, b) =>
    sortOrder === 'asc' ? a.price - b.price : b.price - a.price
  )
}, [products, category, sortOrder])
```

**Correct (filtering only recomputes when relevant):**

```tsx
const filtered = useMemo(
  () => products.filter(p => p.category === category),
  [products, category]
)

const sorted = useMemo(
  () => filtered.toSorted((a, b) =>
    sortOrder === 'asc' ? a.price - b.price : b.price - a.price
  ),
  [filtered, sortOrder]
)
```

Same rule for `useEffect`: one effect per concern.

## Pattern 9 — Extract stable default values for memoized components

Non-primitive default parameters create new references on every render, breaking memoization.

**Incorrect (new function every render):**

```tsx
const UserAvatar = memo(function({ onClick = () => {} }: Props) {
  // ...
})
```

**Correct (stable reference):**

```tsx
const NOOP = () => {}

const UserAvatar = memo(function({ onClick = NOOP }: Props) {
  // ...
})
```

## Pattern 10 — Use useDeferredValue for expensive derived renders

Keep input responsive while heavy computation catches up.

**Incorrect (input feels laggy):**

```tsx
function Search({ items }: { items: Item[] }) {
  const [query, setQuery] = useState('')
  const filtered = items.filter(i => fuzzyMatch(i, query))

  return (
    <>
      <input value={query} onChange={e => setQuery(e.target.value)} />
      <ResultsList results={filtered} />
    </>
  )
}
```

**Correct (input stays snappy):**

```tsx
function Search({ items }: { items: Item[] }) {
  const [query, setQuery] = useState('')
  const deferred = useDeferredValue(query)
  const filtered = useMemo(
    () => items.filter(i => fuzzyMatch(i, deferred)),
    [items, deferred]
  )

  return (
    <>
      <input value={query} onChange={e => setQuery(e.target.value)} />
      <div style={{ opacity: query !== deferred ? 0.7 : 1 }}>
        <ResultsList results={filtered} />
      </div>
    </>
  )
}
```

Wrap the expensive computation in `useMemo` with the deferred value as dependency.

## Pattern 11 — Use startTransition for non-urgent updates

Mark frequent, non-urgent state updates as transitions so React prioritizes urgent work.

**Incorrect (blocks UI on scroll):**

```tsx
function ScrollTracker() {
  const [y, setY] = useState(0)
  useEffect(() => {
    const handler = () => setY(window.scrollY)
    window.addEventListener('scroll', handler, { passive: true })
    return () => window.removeEventListener('scroll', handler)
  }, [])
}
```

**Correct (non-blocking):**

```tsx
function ScrollTracker() {
  const [y, setY] = useState(0)
  useEffect(() => {
    const handler = () => startTransition(() => setY(window.scrollY))
    window.addEventListener('scroll', handler, { passive: true })
    return () => window.removeEventListener('scroll', handler)
  }, [])
}
```

## Pattern 12 — Put interaction logic in event handlers, not effects

If a side effect is triggered by a user action, run it in the handler. Don't model it as state + effect.

**Incorrect (effect re-runs on unrelated changes):**

```tsx
function Form() {
  const [submitted, setSubmitted] = useState(false)
  const theme = useContext(ThemeContext)

  useEffect(() => {
    if (submitted) {
      post('/api/register')
      showToast('Registered', theme)
    }
  }, [submitted, theme])

  return <button onClick={() => setSubmitted(true)}>Submit</button>
}
```

**Correct (do it in the handler):**

```tsx
function Form() {
  const theme = useContext(ThemeContext)

  function handleSubmit() {
    post('/api/register')
    showToast('Registered', theme)
  }

  return <button onClick={handleSubmit}>Submit</button>
}
```

## Pattern 13 — Defer state reads to usage point

Don't subscribe to dynamic state (`searchParams`, `localStorage`) if you only read it inside callbacks.

**Incorrect (subscribes to all searchParams changes):**

```tsx
function ShareButton({ chatId }: { chatId: string }) {
  const searchParams = useSearchParams()
  const handleShare = () => {
    shareChat(chatId, { ref: searchParams.get('ref') })
  }
  return <button onClick={handleShare}>Share</button>
}
```

**Correct (reads on demand, no subscription):**

```tsx
function ShareButton({ chatId }: { chatId: string }) {
  const handleShare = () => {
    const ref = new URLSearchParams(window.location.search).get('ref')
    shareChat(chatId, { ref })
  }
  return <button onClick={handleShare}>Share</button>
}
```

## Pattern 14 — Use useRef for transient values

Values that change frequently but shouldn't trigger re-renders (mouse position, intervals, flags) belong in refs.

**Incorrect (renders on every mouse move):**

```tsx
function Tracker() {
  const [x, setX] = useState(0)
  useEffect(() => {
    const onMove = (e: MouseEvent) => setX(e.clientX)
    window.addEventListener('mousemove', onMove)
    return () => window.removeEventListener('mousemove', onMove)
  }, [])
  return <div style={{ left: x }} />
}
```

**Correct (no re-render for tracking):**

```tsx
function Tracker() {
  const dotRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const onMove = (e: MouseEvent) => {
      dotRef.current?.style.setProperty('transform', `translateX(${e.clientX}px)`)
    }
    window.addEventListener('mousemove', onMove)
    return () => window.removeEventListener('mousemove', onMove)
  }, [])

  return <div ref={dotRef} />
}
```

## Pattern 15 — Subscribe to derived boolean state

Subscribe to boolean conclusions instead of continuous values to reduce re-render frequency.

**Incorrect (re-renders on every pixel):**

```tsx
function Sidebar() {
  const width = useWindowWidth()
  const isMobile = width < 768
  return <nav className={isMobile ? 'mobile' : 'desktop'} />
}
```

**Correct (re-renders only on boolean change):**

```tsx
function Sidebar() {
  const isMobile = useMediaQuery('(max-width: 767px)')
  return <nav className={isMobile ? 'mobile' : 'desktop'} />
}
```

## Advanced Pattern A — Store event handlers in refs for stable subscriptions

When an effect subscribes to a callback that changes often, store the callback in a ref to avoid re-subscription.

**Incorrect (re-subscribes on every render):**

```tsx
function useWindowEvent(event: string, handler: (e: Event) => void) {
  useEffect(() => {
    window.addEventListener(event, handler)
    return () => window.removeEventListener(event, handler)
  }, [event, handler])
}
```

**Correct (stable subscription):**

```tsx
function useWindowEvent(event: string, handler: (e: Event) => void) {
  const handlerRef = useRef(handler)
  useEffect(() => { handlerRef.current = handler }, [handler])

  useEffect(() => {
    const listener = (e: Event) => handlerRef.current(e)
    window.addEventListener(event, listener)
    return () => window.removeEventListener(event, listener)
  }, [event])
}
```

**With `useEffectEvent` (React canary):**

```tsx
function useWindowEvent(event: string, handler: (e: Event) => void) {
  const onEvent = useEffectEvent(handler)
  useEffect(() => {
    window.addEventListener(event, onEvent)
    return () => window.removeEventListener(event, onEvent)
  }, [event])
}
```

## Advanced Pattern B — Do not put Effect Events in dependency arrays

`useEffectEvent` returns a function with intentionally unstable identity. Including it in dependencies makes the effect re-run every render.

**Incorrect:**

```tsx
const onConnected = useEffectEvent(onConnectedProp)
useEffect(() => {
  connection.on('connected', onConnected)
}, [onConnected])  // Re-runs every render
```

**Correct:**

```tsx
const onConnected = useEffectEvent(onConnectedProp)
useEffect(() => {
  connection.on('connected', onConnected)
}, [])  // Only reactive values in deps
```

## Decision table — re-render fix by symptom

| Symptom | Fix | Key technique |
|---|---|---|
| Child re-renders when parent changes | Extract + memo | `memo()` child, early return |
| Component defined inside another | Move to module scope | Pass props instead |
| State updated in effect from props | Derive during render | Compute from props/state directly |
| Stale closure in callback | Functional setState | `setX(curr => ...)` |
| Expensive init runs every render | Lazy initialization | `useState(() => ...)` |
| `useMemo` on simple boolean | Remove memo | Direct expression |
| Effect runs too often | Narrow deps | Primitive values, derived booleans |
| Sort changes trigger filter recompute | Split hooks | Separate `useMemo` per concern |
| Memoized component with default object | Extract constant | Module-level stable reference |
| Input lag during search | Deferred value | `useDeferredValue` + `useMemo` |
| Scroll/resize blocks UI | Transition | `startTransition` |
| Action modeled as state+effect | Event handler | Move logic to `onClick`/`onSubmit` |
| `useSearchParams` only in callback | Direct read | `window.location.search` |
| Frequent updates without re-render | Ref | `useRef` + DOM mutation |
| Width changes trigger re-render | Media query | `useMediaQuery('(max-width: ...)')` |

## When NOT to optimize

- **React Compiler enabled:** The compiler handles memoization automatically. Adding manual `memo()`/`useMemo()` is redundant.
- **Premature optimization:** If a component renders <1ms and re-renders infrequently, memoization adds complexity with no benefit.
- **State that must trigger re-renders:** Don't put UI-visible state in refs — updates won't render.

## Common failure modes

| Mistake | Result |
|---|---|
| Inline objects/arrays in `useEffect` deps | Effect runs every render |
| Missing dependency in `useCallback` | Stale closure, uses old state |
| `useMemo` without dependency array | Runs every render anyway |
| `memo()` on component with non-primitive default | Memoization silently broken |
| `useDeferredValue` without `useMemo` | Still runs expensive computation every render |
| `useEffectEvent` in dependency array | Effect re-runs every render, lint error |
| `useRef` for UI-visible value | Updates don't trigger re-render |
