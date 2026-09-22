---
name: react-js-performance
description: Apply JavaScript micro-optimizations for hot paths in React applications.
license: MIT
last_reviewed: 2026-05-02
---

# JavaScript performance in React — micro-optimizations for hot paths

## When to apply

- Array operations show up in profiler as hot paths
- Repeated `.find()` or `.includes()` in loops
- DOM manipulation causes forced reflows
- `localStorage` or `document.cookie` read on every call
- Multiple `.filter()`/`.map()` chains iterate the same array
- `.sort()` mutates React state arrays

## Pattern 1 — Avoid layout thrashing

Interleaving style writes with layout reads forces synchronous reflows.

**Incorrect (forces reflow per read):**

```typescript
element.style.width = '100px'
const w = element.offsetWidth  // Forces reflow
element.style.height = '200px'
const h = element.offsetHeight  // Forces another reflow
```

**Correct (batch writes, then read):**

```typescript
element.style.width = '100px'
element.style.height = '200px'
element.style.backgroundColor = 'blue'
const { width, height } = element.getBoundingClientRect()  // Single reflow
```

**Better: use CSS classes:**

```tsx
// Toggle class instead of inline styles
function Box({ highlighted }: { highlighted: boolean }) {
  return <div className={highlighted ? 'highlighted-box' : ''}>Content</div>
}
```

See [CSS Triggers](https://csstriggers.com/) for which properties force layout.

## Pattern 2 — Use Set/Map for O(1) lookups

Convert arrays to Set/Map for repeated membership checks.

**Incorrect (O(n) per check):**

```typescript
const allowedIds = ['a', 'b', 'c']
items.filter(item => allowedIds.includes(item.id))
```

**Correct (O(1) per check):**

```typescript
const allowedIds = new Set(['a', 'b', 'c'])
items.filter(item => allowedIds.has(item.id))
```

## Pattern 3 — Build index maps for repeated lookups

Multiple `.find()` calls by the same key should use a Map.

**Incorrect (O(n) per lookup):**

```typescript
function processOrders(orders: Order[], users: User[]) {
  return orders.map(order => ({
    ...order,
    user: users.find(u => u.id === order.userId)
  }))
}
```

**Correct (O(1) per lookup):**

```typescript
function processOrders(orders: Order[], users: User[]) {
  const userById = new Map(users.map(u => [u.id, u]))
  return orders.map(order => ({
    ...order,
    user: userById.get(order.userId)
  }))
}
```

For 1000 orders x 1000 users: 1M ops → 2K ops.

## Pattern 4 — Cache storage API calls

`localStorage`, `sessionStorage`, and `document.cookie` are synchronous and expensive.

**Incorrect (reads storage on every call):**

```typescript
function getTheme() {
  return localStorage.getItem('theme') ?? 'light'
}
// 10 calls = 10 storage reads
```

**Correct (Map cache with invalidation):**

```typescript
const storageCache = new Map<string, string | null>()

function getLocalStorage(key: string) {
  if (!storageCache.has(key)) {
    storageCache.set(key, localStorage.getItem(key))
  }
  return storageCache.get(key)
}

function setLocalStorage(key: string, value: string) {
  localStorage.setItem(key, value)
  storageCache.set(key, value)
}

// Invalidate on external changes
window.addEventListener('storage', e => {
  if (e.key) storageCache.delete(e.key)
})
```

Use a Map (not a hook) so it works in utilities and event handlers outside React components.

## Pattern 5 — Cache repeated function calls

Use a module-level Map for expensive pure functions called with the same inputs.

**Incorrect (redundant computation):**

```tsx
function ProjectList({ projects }: { projects: Project[] }) {
  return (
    <div>
      {projects.map(p => (
        <ProjectCard key={p.id} slug={slugify(p.name)} />
      ))}
    </div>
  )
}
```

**Correct (cached results):**

```typescript
const slugifyCache = new Map<string, string>()

function cachedSlugify(text: string): string {
  if (slugifyCache.has(text)) return slugifyCache.get(text)!
  const result = slugify(text)
  slugifyCache.set(text, result)
  return result
}
```

## Pattern 6 — Combine multiple array iterations

Multiple `.filter()` or `.map()` calls iterate multiple times. Combine into one loop.

**Incorrect (3 iterations):**

```typescript
const admins = users.filter(u => u.isAdmin)
const testers = users.filter(u => u.isTester)
const inactive = users.filter(u => !u.isActive)
```

**Correct (1 iteration):**

```typescript
const admins: User[] = []
const testers: User[] = []
const inactive: User[] = []

for (const user of users) {
  if (user.isAdmin) admins.push(user)
  if (user.isTester) testers.push(user)
  if (!user.isActive) inactive.push(user)
}
```

## Pattern 7 — Use flatMap for map + filter

`.map().filter(Boolean)` creates an intermediate array and iterates twice.

**Incorrect (2 iterations, intermediate array):**

```typescript
const names = users
  .map(u => u.isActive ? u.name : null)
  .filter(Boolean)
```

**Correct (1 iteration, no intermediate):**

```typescript
const names = users.flatMap(u =>
  u.isActive ? [u.name] : []
)
```

## Pattern 8 — Use toSorted() instead of sort() for immutability

`.sort()` mutates in place, breaking React's immutability expectations.

**Incorrect (mutates prop array):**

```tsx
const sorted = useMemo(
  () => users.sort((a, b) => a.name.localeCompare(b.name)),
  [users]
)
```

**Correct (creates new array):**

```tsx
const sorted = useMemo(
  () => users.toSorted((a, b) => a.name.localeCompare(b.name)),
  [users]
)
```

Browser support: Chrome 110+, Safari 16+, Firefox 115+, Node.js 20+. Fallback: `[...items].sort(...)`.

Other immutable methods: `.toReversed()`, `.toSpliced()`, `.with()`.

## Pattern 9 — Use loop for min/max instead of sort

Sorting to find the smallest or largest element is O(n log n) when O(n) suffices.

**Incorrect (O(n log n)):**

```typescript
function getLatest(projects: Project[]) {
  return [...projects].sort((a, b) => b.updatedAt - a.updatedAt)[0]
}
```

**Correct (O(n)):**

```typescript
function getLatest(projects: Project[]) {
  if (projects.length === 0) return null
  let latest = projects[0]
  for (let i = 1; i < projects.length; i++) {
    if (projects[i].updatedAt > latest.updatedAt) latest = projects[i]
  }
  return latest
}
```

For small arrays, `Math.min(...numbers)` works but can throw on very large arrays due to spread limitations.

## Pattern 10 — Early return and early length check

Return early when the result is determined. Check array lengths before expensive comparisons.

**Incorrect (sorts even when lengths differ):**

```typescript
function hasChanges(a: string[], b: string[]) {
  return a.sort().join() !== b.sort().join()  // Mutates originals too
}
```

**Correct (length check + no mutation):**

```typescript
function hasChanges(a: string[], b: string[]) {
  if (a.length !== b.length) return true
  const as = a.toSorted()
  const bs = b.toSorted()
  for (let i = 0; i < as.length; i++) {
    if (as[i] !== bs[i]) return true
  }
  return false
}
```

## Pattern 11 — Cache property access in loops

Cache deeply nested property lookups and array lengths in hot paths.

**Incorrect (3 lookups x N iterations):**

```typescript
for (let i = 0; i < arr.length; i++) {
  process(obj.config.settings.value)
}
```

**Correct (1 lookup total):**

```typescript
const value = obj.config.settings.value
const len = arr.length
for (let i = 0; i < len; i++) {
  process(value)
}
```

## Pattern 12 — Hoist RegExp creation

Creating RegExp inside render or loops allocates on every invocation.

**Incorrect (new RegExp every render):**

```tsx
function Highlighter({ text, query }: Props) {
  const regex = new RegExp(`(${query})`, 'gi')
  const parts = text.split(regex)
  // ...
}
```

**Correct (hoist static, memoize dynamic):**

```tsx
const EMAIL_REGEX = /^[^\s@]+@[^\s@]+\.[^\s@]+$/

function Highlighter({ text, query }: Props) {
  const regex = useMemo(
    () => new RegExp(`(${escapeRegex(query)})`, 'gi'),
    [query]
  )
  const parts = text.split(regex)
  // ...
}
```

**Warning:** Global regex (`/g`) has mutable `lastIndex`. Don't cache global patterns without resetting `lastIndex`.

## Pattern 13 — Defer non-critical work with requestIdleCallback

Schedule analytics, localStorage writes, and prefetching during browser idle periods.

**Incorrect (blocks main thread):**

```typescript
function handleSearch(query: string) {
  setResults(searchItems(query))
  analytics.track('search', { query })
  saveToRecentSearches(query)
}
```

**Correct (defers to idle time):**

```typescript
function handleSearch(query: string) {
  setResults(searchItems(query))
  requestIdleCallback(() => analytics.track('search', { query }))
  requestIdleCallback(() => saveToRecentSearches(query))
}
```

With timeout for required work: `requestIdleCallback(fn, { timeout: 2000 })`.

Fallback for unsupported browsers: `window.requestIdleCallback ?? (cb => setTimeout(cb, 1))`.

**Do NOT use for:** user-initiated actions needing immediate feedback, rendering updates the user is waiting for, time-sensitive operations.

## Pattern 14 — Chunk large tasks with requestIdleCallback

Process large datasets in small chunks to keep the main thread responsive.

```typescript
function processLargeDataset(items: Item[]) {
  let index = 0

  function processChunk(deadline: IdleDeadline) {
    while (index < items.length && deadline.timeRemaining() > 0) {
      processItem(items[index])
      index++
    }
    if (index < items.length) {
      requestIdleCallback(processChunk)
    }
  }

  requestIdleCallback(processChunk)
}
```

## Decision table — JS optimization by symptom

| Symptom | Fix | Complexity |
|---|---|---|
| DOM reads after writes | Batch reads/writes | Read all, then write all |
| `includes()` in filter loop | Set lookup | `new Set()` + `.has()` |
| Repeated `.find()` by same key | Index map | `new Map(array.map(...))` |
| `localStorage.getItem` on every call | Memory cache | `Map` with invalidation |
| Pure function called with same args | Memoize | Module-level `Map` |
| Chained `.filter().map()` | flatMap or for-loop | Single iteration |
| `.sort()` on React state/props | Immutable sort | `.toSorted()` |
| Sort to find min/max | Single loop | O(n) scan |
| Array comparison with sort+join | Early length check + loop | O(1) guard, O(n) compare |
| Deep property in hot loop | Cache lookup | Hoist before loop |
| `new RegExp()` in render | Hoist/memoize | Module const or `useMemo` |
| Analytics blocks interaction | requestIdleCallback | Defer non-critical work |
| Large dataset blocks thread | Chunked processing | `requestIdleCallback` loop |

## When NOT to micro-optimize

- **One-time operations:** Setup code, initialization, user-triggered actions with clear feedback.
- **Small datasets (<100 items):** The difference between O(n) and O(n log n) is negligible.
- **Readability tradeoff:** A `for` loop that's 3x faster but 10x harder to read isn't worth it unless it's in a verified hot path.

## Common failure modes

| Mistake | Result |
|---|---|
| Interleaved DOM read/write | Forced synchronous reflow, jank |
| `Set` created inside render loop | Recreated every render, no benefit |
| Storage cache without invalidation | Stale data when other tabs change storage |
| Global regex cached without reset | `lastIndex` pollution, intermittent failures |
| `requestIdleCallback` for critical work | User sees delayed feedback |
| `sort()` on React props | Mutates parent state, breaks React assumptions |
| `Math.min(...largeArray)` | Spread limit exceeded, runtime error |
