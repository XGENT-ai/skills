---
name: react-data-fetching
description: Eliminate async waterfalls and optimize client-side data fetching in React.
license: MIT
last_reviewed: 2026-09-26
---

# React data fetching — waterfall elimination and client-side patterns

## When to apply

- Page loads trigger 3+ sequential network requests
- Multiple components fetch the same endpoint independently
- Scroll or touch interactions feel janky due to blocking event listeners
- `localStorage` schema changes break existing user data
- Independent async operations are awaited one after another

## Pattern 1 — Parallelize independent fetches with Promise.all()

**Incorrect (sequential, N round trips):**

```typescript
const user = await fetchUser()
const posts = await fetchPosts()
const comments = await fetchComments()
```

**Correct (parallel, 1 round trip):**

```typescript
const [user, posts, comments] = await Promise.all([
  fetchUser(),
  fetchPosts(),
  fetchComments()
])
```

## Pattern 2 — Chain dependent fetches per item

When nested data has per-item dependencies, chain inside each promise so a slow item doesn't block the rest.

**Incorrect (a single slow item blocks all nested fetches):**

```typescript
const chats = await Promise.all(chatIds.map(id => getChat(id)))
const authors = await Promise.all(chats.map(c => getUser(c.author)))
```

**Correct (each item chains independently):**

```typescript
const authors = await Promise.all(
  chatIds.map(id => getChat(id).then(c => getUser(c.author)))
)
```

## Pattern 3 — Defer await to the branch that needs it

Move `await` into the code path that consumes the result. Don't pay for async work on branches that return early.

**Incorrect (blocks even on early return):**

```typescript
async function handle(userId: string, skip: boolean) {
  const user = await fetchUser(userId)  // Paid even when skip=true
  if (skip) return { skipped: true }
  return process(user)
}
```

**Correct (only fetches when needed):**

```typescript
async function handle(userId: string, skip: boolean) {
  if (skip) return { skipped: true }
  const user = await fetchUser(userId)
  return process(user)
}
```

## Pattern 4 — Check cheap conditions before expensive async flags

When a compound condition mixes a cheap synchronous check with an async flag, evaluate the cheap check first.

**Incorrect (always pays async cost):**

```typescript
const flag = await getFlag()
if (flag && cheapCondition) { /* ... */ }
```

**Correct (skips async when cheap check fails):**

```typescript
if (cheapCondition) {
  const flag = await getFlag()
  if (flag) { /* ... */ }
}
```

Keep the original order if the cheap condition is expensive, depends on the flag, or side effects must run in fixed order.

## Pattern 5 — Use Suspense boundaries for non-critical data

A loading gate at the top of the tree blocks everything below it, including parts that need no data. Put the boundary around the data-dependent subtree so the shell paints immediately.

**Incorrect (one pending request hides the whole page):**

```tsx
function Page() {
  const { data, isLoading } = useData()
  if (isLoading) return <FullPageSpinner />  // Sidebar, Header, Footer all wait
  return (
    <div>
      <Sidebar />
      <Header />
      <DataDisplay data={data} />
      <Footer />
    </div>
  )
}
```

**Correct (shell renders immediately, only the subtree suspends):**

```tsx
function Page() {
  return (
    <div>
      <Sidebar />
      <Header />
      <Suspense fallback={<Skeleton />}>
        <DataDisplay dataPromise={dataPromise} />
      </Suspense>
      <Footer />
    </div>
  )
}

function DataDisplay({ dataPromise }: { dataPromise: Promise<Data> }) {
  const data = use(dataPromise)  // React 19
  return <div>{data.content}</div>
}
```

**Never create the promise during render of the suspending component.** A new promise each render means the component suspends forever. The promise must come from a stable source: module scope, a router loader, a cache, or a Suspense-enabled data library.

Two ways the render-time promise sneaks back in:

- **`.then()` returns a new promise.** `use(getUser(id).then(u => u.profile))` is recreated every render even when `getUser` is cached. Unwrap first, then transform: `const profile = use(getUser(id)).profile`.
- **`useMemo` doesn't make it stable on first mount.** React keeps no state for a component that suspends before it first mounts, so the memoized promise is rebuilt on every retry.

With a Suspense-enabled library there is no promise to manage — `useSuspenseQuery({ queryKey: ['user', id], queryFn })` from TanStack Query suspends the nearest boundary directly.

**Start promises above the boundaries.** Sibling boundaries that receive already-started promises load in parallel. A component that `use()`s one promise before rendering the child that starts the next request is a waterfall — Pattern 1 in component form.

**Rejections go to an error boundary.** `use` can't be wrapped in `try`/`catch`; put an error boundary around the Suspense boundary:

```tsx
<ErrorBoundary fallback={<LoadError />}>
  <Suspense fallback={<Skeleton />}>
    <DataDisplay dataPromise={dataPromise} />
  </Suspense>
</ErrorBoundary>
```

`use` is not a Hook: it can be called inside conditions and loops, and after an early return. That includes `use(ThemeContext)` as a conditional replacement for `useContext`.

Moving an effect-based fetch (`useEffect` + `useState`) to `use()` only pays off once a stable promise source exists. Until then, keep the effect and its [`ignore` flag](effect-patterns.md#cleanup-rule-2--data-fetching-needs-an-ignore-flag).

**Do NOT use Suspense when:**

- The data is needed for layout decisions (causes layout shift)
- The content is SEO-critical and above the fold
- The query is small and fast enough that Suspense overhead isn't worth it

## Pattern 6 — Deduplicate client requests with SWR

Multiple component instances fetching the same endpoint should share one request.

**Incorrect (N instances = N requests):**

```tsx
function UserList() {
  const [users, setUsers] = useState([])
  useEffect(() => {
    fetch('/api/users').then(r => r.json()).then(setUsers)
  }, [])
}
```

**Correct (instances share one request):**

```tsx
import useSWR from 'swr'

function UserList() {
  const { data: users } = useSWR('/api/users', fetcher)
}
```

## Pattern 7 — Use passive event listeners for scroll/touch

Browsers wait for non-passive listeners to finish before scrolling, causing jank. Add `{ passive: true }` when the listener never calls `preventDefault()`.

**Incorrect (blocks scroll):**

```typescript
window.addEventListener('touchstart', handler)
window.addEventListener('wheel', handler)
```

**Correct (scrolls immediately):**

```typescript
window.addEventListener('touchstart', handler, { passive: true })
window.addEventListener('wheel', handler, { passive: true })
```

**Do NOT use passive when** the listener calls `preventDefault()` (custom swipe gestures, zoom controls).

## Pattern 8 — Version and minimize localStorage data

Unversioned keys break when schema changes. Store only needed fields and wrap in try-catch.

**Incorrect:**

```typescript
localStorage.setItem('userConfig', JSON.stringify(fullUserObject))
const data = localStorage.getItem('userConfig')  // Throws in private browsing
```

**Correct:**

```typescript
const VERSION = 'v2'

function saveConfig(config: { theme: string; language: string }) {
  try {
    localStorage.setItem(`userConfig:${VERSION}`, JSON.stringify(config))
  } catch { /* quota exceeded or private browsing */ }
}

function loadConfig() {
  try {
    const data = localStorage.getItem(`userConfig:${VERSION}`)
    return data ? JSON.parse(data) : null
  } catch { return null }
}
```

## Decision table — which pattern for which situation

| Symptom | Pattern | Key change |
|---|---|---|
| 3+ sequential `await` calls | Promise.all() | Start all promises together |
| Nested data, one slow item blocks rest | Per-item chaining | `map(id => a(id).then(b))` |
| Early return after await | Defer await | Move `await` past the guard |
| `flag && cheapCondition` with async flag | Cheap-first | Swap order |
| Layout blocked by data fetch | Suspense boundary | `use()` + `<Suspense>` |
| Same endpoint fetched in multiple components | SWR | `useSWR(key, fetcher)` |
| Scroll/touch feels janky | Passive listeners | `{ passive: true }` |
| localStorage schema drift or crashes | Versioned keys | `key:vN` + try-catch |

## When NOT to parallelize

- **Dependencies exist:** If `fetchB()` needs the result of `fetchA()`, use chaining or `better-all` for dependency-based parallelization.
- **Side effect ordering matters:** Auth checks that must run before data fetches for audit logging.
- **Resource limits:** The browser caps concurrent requests per domain (typically 6). Parallelizing 20 fetches to the same origin just queues them anyway.

## Common failure modes

| Mistake | Result |
|---|---|
| `await` inside `.map()` without `Promise.all()` | Sequential execution disguised as iteration |
| Forgetting to start promises before `await` | Still sequential even with `Promise.all()` |
| `&&` conditional rendering with `count` value | Renders `0` instead of nothing (use `> 0 ? ... : null`) |
| Non-passive listeners on scroll/touch | Janky scrolling, missed frames |
| Promise created in render (incl. `.then()` or `useMemo`) passed to `use()` | Fallback shows repeatedly; content never appears |
