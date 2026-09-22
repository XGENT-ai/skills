---
name: react-rendering-performance
description: Optimize browser rendering, hydration, and resource loading in React applications.
license: MIT
last_reviewed: 2026-05-02
---

# React rendering performance — browser paint, hydration, and resources

## When to apply

- Long lists cause slow initial paint
- Hydration warnings flood the console
- Theme switching causes visible flicker
- SVG animations are janky
- External scripts block rendering
- Critical fonts/styles load late

## Pattern 1 — CSS content-visibility for long lists

Apply `content-visibility: auto` to defer off-screen layout and paint work.

```css
.message-item {
  content-visibility: auto;
  contain-intrinsic-size: 0 80px;
}
```

```tsx
function MessageList({ messages }: { messages: Message[] }) {
  return (
    <div className="overflow-y-auto h-screen">
      {messages.map(msg => (
        <div key={msg.id} className="message-item">
          <Avatar user={msg.author} />
          <div>{msg.content}</div>
        </div>
      ))}
    </div>
  )
}
```

For 1000 items, the browser skips layout/paint for ~990 off-screen items. `contain-intrinsic-size` prevents scrollbar jumping by reserving estimated space.

**Do NOT use when:** Items are all above the fold, or item heights vary wildly and `contain-intrinsic-size` causes excessive scrollbar jitter.

## Pattern 2 — Prevent hydration mismatch without flickering

Client-only data (localStorage, cookies) causes SSR/hydration mismatches. A synchronous inline script updates the DOM before React hydrates.

**Incorrect (breaks SSR):**

```tsx
function ThemeWrapper({ children }: { children: ReactNode }) {
  const theme = localStorage.getItem('theme') || 'light'  // Throws on server
  return <div className={theme}>{children}</div>
}
```

**Incorrect (flickers):**

```tsx
function ThemeWrapper({ children }: { children: ReactNode }) {
  const [theme, setTheme] = useState('light')
  useEffect(() => {
    setTheme(localStorage.getItem('theme') || 'light')  // Flash of wrong theme
  }, [])
  return <div className={theme}>{children}</div>
}
```

**Correct (no flicker, no mismatch):**

```tsx
function ThemeWrapper({ children }: { children: ReactNode }) {
  return (
    <>
      <div id="theme-wrapper">{children}</div>
      <script
        dangerouslySetInnerHTML={{
          __html: `(function(){
            try {
              var t = localStorage.getItem('theme') || 'light';
              var el = document.getElementById('theme-wrapper');
              if (el) el.className = t;
            } catch(e) {}
          })();`
        }}
      />
    </>
  )
}
```

The inline script executes synchronously before hydration. No visible flash.

## Pattern 3 — Suppress expected hydration mismatches

Some values are intentionally different on server vs client (dates, random IDs, locale formatting). Wrap these in `suppressHydrationWarning`.

**Incorrect (noisy console warnings):**

```tsx
function Timestamp() {
  return <span>{new Date().toLocaleString()}</span>
}
```

**Correct (suppresses known mismatch):**

```tsx
function Timestamp() {
  return <span suppressHydrationWarning>{new Date().toLocaleString()}</span>
}
```

**Do NOT use this to hide real bugs.** Only for values that are guaranteed to differ between server and client.

## Pattern 4 — Use React DOM resource hints

Declare critical resources as early as possible in the tree. These `react-dom` APIs are safe to call during render — React deduplicates repeated calls and emits the corresponding `<link>` tags.

```tsx
import { preconnect, prefetchDNS, preload, preinit } from 'react-dom'

function App({ children }) {
  prefetchDNS('https://analytics.example.com')
  preconnect('https://api.example.com')
  preload('/fonts/inter.woff2', { as: 'font', type: 'font/woff2', crossOrigin: 'anonymous' })
  preinit('/styles/critical.css', { as: 'style' })

  return <Shell>{children}</Shell>
}
```

| API | Use case |
|---|---|
| `prefetchDNS` | Third-party domains you'll connect to later |
| `preconnect` | APIs or CDNs you'll fetch from immediately |
| `preload` | Critical resources needed for current page |
| `preinit` | Stylesheets/scripts that must execute early |
| `preloadModule` | JS modules for likely next navigation |

## Pattern 5 — Use defer or async on script tags

Scripts without `defer` or `async` block HTML parsing.

**Incorrect (blocks rendering):**

```tsx
<script src="https://example.com/analytics.js" />
<script src="/scripts/utils.js" />
```

**Correct:**

```tsx
<script src="https://example.com/analytics.js" async />
<script src="/scripts/utils.js" defer />
```

| Attribute | Behavior | Use for |
|---|---|---|
| `defer` | Download in parallel, execute after HTML parse, maintain order | DOM-dependent scripts, scripts with dependencies |
| `async` | Download in parallel, execute when ready, no order guarantee | Independent scripts (analytics, ads) |

## Pattern 6 — Animate SVG wrapper, not SVG element

Most browsers lack GPU acceleration for CSS animations on SVG elements. Animate a wrapper `div` instead.

**Incorrect (no hardware acceleration):**

```tsx
<svg className="animate-spin" width="24" height="24">
  <circle cx="12" cy="12" r="10" stroke="currentColor" />
</svg>
```

**Correct (GPU accelerated):**

```tsx
<div className="animate-spin">
  <svg width="24" height="24">
    <circle cx="12" cy="12" r="10" stroke="currentColor" />
  </svg>
</div>
```

Applies to all CSS transforms: `transform`, `opacity`, `translate`, `scale`, `rotate`.

## Pattern 7 — Hoist static JSX elements

Extract static JSX outside components to avoid recreation on every render.

**Incorrect (recreates every render):**

```tsx
function LoadingSkeleton() {
  return <div className="animate-pulse h-20 bg-gray-200" />
}

function Container() {
  return <div>{loading && <LoadingSkeleton />}</div>
}
```

**Correct (reuses same element):**

```tsx
const loadingSkeleton = <div className="animate-pulse h-20 bg-gray-200" />

function Container() {
  return <div>{loading && loadingSkeleton}</div>
}
```

Especially helpful for large static SVG nodes.

> React Compiler automatically hoists static JSX. Manual hoisting is only needed without the compiler.

## Pattern 8 — Use explicit conditional rendering

`&&` with numeric conditions renders `0` or `NaN`. Use ternary for conditions that can be falsy primitives.

**Incorrect (renders "0"):**

```tsx
function Badge({ count }: { count: number }) {
  return <div>{count && <span className="badge">{count}</span>}</div>
}
```

**Correct (renders nothing when 0):**

```tsx
function Badge({ count }: { count: number }) {
  return <div>{count > 0 ? <span className="badge">{count}</span> : null}</div>
}
```

## Pattern 9 — Use Activity component for show/hide

Preserve state and DOM for expensive components that frequently toggle visibility.

```tsx
import { Activity } from 'react'

function Dropdown({ isOpen }: Props) {
  return (
    <Activity mode={isOpen ? 'visible' : 'hidden'}>
      <ExpensiveMenu />
    </Activity>
  )
}
```

Avoids expensive unmount/remount cycles while hiding the component from the layout.

## Pattern 10 — Use useTransition over manual loading states

`useTransition` provides automatic `isPending` state without manual `setIsLoading` boilerplate.

**Incorrect (manual loading state):**

```tsx
function Search() {
  const [query, setQuery] = useState('')
  const [results, setResults] = useState([])
  const [isLoading, setIsLoading] = useState(false)

  const handleSearch = async (value: string) => {
    setIsLoading(true)
    setQuery(value)
    const data = await fetchResults(value)
    setResults(data)
    setIsLoading(false)
  }
}
```

**Correct (built-in pending state):**

```tsx
function Search() {
  const [query, setQuery] = useState('')
  const [results, setResults] = useState([])
  const [isPending, startTransition] = useTransition()

  const handleSearch = (value: string) => {
    setQuery(value)
    startTransition(async () => {
      const data = await fetchResults(value)
      setResults(data)
    })
  }
}
```

Benefits: automatic pending state, error resilience, interrupt handling (new transitions cancel pending ones).

## Pattern 11 — Optimize SVG precision

Reduce SVG coordinate precision to decrease file size.

**Incorrect (excessive precision):**

```svg
<path d="M 10.293847 20.847362 L 30.938472 40.192837" />
```

**Correct (1 decimal place):**

```svg
<path d="M 10.3 20.8 L 30.9 40.2" />
```

Automate with `npx svgo --precision=1 --multipass icon.svg`.

## Decision table — rendering fix by symptom

| Symptom | Fix | Key technique |
|---|---|---|
| Long list slow initial paint | content-visibility | `content-visibility: auto` + `contain-intrinsic-size` |
| Theme flicker on load | Sync inline script | `dangerouslySetInnerHTML` script before hydration |
| Date/random ID hydration warnings | suppressHydrationWarning | Wrap expected mismatch only |
| Critical fonts load late | Resource hints | `preload()` near the root of the tree |
| Scripts block rendering | defer/async | `defer` for ordered, `async` for independent |
| SVG animation jank | Wrapper animation | Animate `div` wrapper, not SVG element |
| Static JSX recreated | Hoist to module | Define outside component |
| `0` renders instead of nothing | Explicit ternary | `count > 0 ? ... : null` |
| Expensive toggle unmounts | Activity component | `<Activity mode="visible"|"hidden">` |
| Manual loading state boilerplate | useTransition | `startTransition` + `isPending` |
| Large SVG file size | Precision reduction | `svgo --precision=1` |

## When NOT to apply

- **No SSR:** Hydration patterns are irrelevant for client-only renders (CRA, Vite SPA without SSR).
- **Short lists (<50 items):** `content-visibility` overhead isn't worth it.
- **All content above the fold:** Nothing is off-screen to skip.

## Common failure modes

| Mistake | Result |
|---|---|
| `content-visibility` without `contain-intrinsic-size` | Scrollbar jumps, layout shift |
| `suppressHydrationWarning` on real bugs | Silent hydration mismatches |
| `async` on dependent scripts | Race conditions, undefined variables |
| `dangerouslySetInnerHTML` without try-catch | Script error breaks hydration |
| SVG precision too aggressive | Visual distortion |
