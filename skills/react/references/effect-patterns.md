---
name: react-effect-patterns
description: Avoid Effect anti-patterns, missing deps, and stale subscriptions in React.
license: MIT
last_reviewed: 2026-05-02
---

# React effect patterns — correctness, dependencies, and cleanup

## When to apply

- A `useEffect` resets state when a prop changes
- Parent needs to know about a child state change
- A child passes fetched data up to its parent
- Multiple effects chain together via shared state
- `eslint-disable react-hooks/exhaustive-deps` appears in the codebase
- An object or function in the dependency array causes the effect to re-run every render
- Subscriptions are added without a cleanup return
- An effect "adjusts" state when a list or object prop changes
- Data fetching inside `useEffect` shows stale results on fast navigation

## The question to ask first

Effects are an escape hatch for synchronizing with **external systems**. If no external system is involved, the Effect is probably the wrong tool. Before writing one, check whether the value can be calculated during render (see [derived state](rerender-optimization.md#pattern-3--calculate-derived-state-during-render)) or the logic belongs in an [event handler](rerender-optimization.md#pattern-12--put-interaction-logic-in-event-handlers-not-effects).

## Anti-pattern 1 — Reset state via effect; use `key` instead

**Wrong (extra render, brittle):**

```tsx
function Profile({ userId }) {
  const [comment, setComment] = useState('');
  useEffect(() => {
    setComment('');
  }, [userId]);
  return <Comment comment={comment} onChange={setComment} />;
}
```

**Correct (React resets state automatically when key changes):**

```tsx
function ProfilePage({ userId }) {
  return <Profile userId={userId} key={userId} />;
}

function Profile({ userId }) {
  const [comment, setComment] = useState(''); // resets on mount
}
```

Use `key` any time you want a component to start fresh when an identity prop changes.

## Anti-pattern 2 — Notify parent via effect; update both in the handler

**Wrong (double render, effect fires after paint):**

```tsx
function Toggle({ onChange }) {
  const [isOn, setIsOn] = useState(false);
  useEffect(() => {
    onChange(isOn);
  }, [isOn, onChange]);
}
```

**Correct (single update, no extra render):**

```tsx
function Toggle({ onChange }) {
  const [isOn, setIsOn] = useState(false);
  function handleClick() {
    const next = !isOn;
    setIsOn(next);
    onChange(next);
  }
  return <button onClick={handleClick}>{isOn ? 'On' : 'Off'}</button>;
}
```

**Best (fully controlled — lift state to parent):**

```tsx
function Toggle({ isOn, onChange }) {
  return <button onClick={() => onChange(!isOn)}>{isOn ? 'On' : 'Off'}</button>;
}
```

## Anti-pattern 3 — Effect chains; merge logic into one event handler

Each effect in a chain causes a re-render before the next one fires. With N effects, you get N render passes.

**Wrong (3 renders per card placement):**

```tsx
useEffect(() => {
  if (card !== null && card.gold) setGoldCardCount(c => c + 1);
}, [card]);

useEffect(() => {
  if (goldCardCount > 3) { setRound(r => r + 1); setGoldCardCount(0); }
}, [goldCardCount]);
```

**Correct (one render, one handler):**

```tsx
const isGameOver = round > 5;

function handlePlaceCard(nextCard) {
  setCard(nextCard);
  if (nextCard.gold) {
    if (goldCardCount < 3) {
      setGoldCardCount(goldCardCount + 1);
    } else {
      setGoldCardCount(0);
      setRound(round + 1);
    }
  }
}
```

Move all the interacting state updates into the event handler that triggers them.

## Anti-pattern 4 — Passing data up via effect; fetch in the parent instead

Data should flow down. A child that fetches and then pushes the result upward through an effect makes the data flow impossible to follow and adds a render pass.

**Wrong (child fetches, parent receives via effect):**

```tsx
function Parent() {
  const [data, setData] = useState(null);
  return <Child onFetched={setData} />;
}

function Child({ onFetched }) {
  const data = useSomeAPI();
  useEffect(() => {
    if (data) onFetched(data);
  }, [onFetched, data]);
}
```

**Correct (parent fetches, passes down):**

```tsx
function Parent() {
  const data = useSomeAPI();
  return <Child data={data} />;
}
```

## Anti-pattern 5 — Adjusting state on prop change; store an ID and derive

When an effect exists only to "fix up" state after a list or object prop changes, store the identity instead and derive the object during render.

**Wrong (selection is wiped whenever `items` changes):**

```tsx
function List({ items }) {
  const [selection, setSelection] = useState(null);
  useEffect(() => {
    setSelection(null);
  }, [items]);
}
```

**Correct (selection survives if the item is still there):**

```tsx
function List({ items }) {
  const [selectedId, setSelectedId] = useState(null);
  const selection = items.find(item => item.id === selectedId) ?? null;
}
```

This is the general fix for "reset some state" — `key` is the fix for "reset all state".

## Dependency rule 1 — Never suppress the exhaustive-deps linter

`eslint-disable-next-line react-hooks/exhaustive-deps` almost always means the effect has a stale closure bug.

**Wrong (setCount uses a stale `count` and `increment`):**

```tsx
useEffect(() => {
  const id = setInterval(() => {
    setCount(count + increment);
  }, 1000);
  return () => clearInterval(id);
  // eslint-disable-next-line react-hooks/exhaustive-deps
}, []);
```

**Correct (fix the code, not the linter):**

```tsx
useEffect(() => {
  const id = setInterval(() => {
    setCount(c => c + increment); // functional update removes `count` dependency
  }, 1000);
  return () => clearInterval(id);
}, [increment]);
```

If a dep is legitimately non-reactive (a stable callback), use `useEffectEvent` — not the lint comment.

## Dependency rule 2 — Move objects and functions inside the effect

Objects and functions created during render get a new reference on every render. Putting them in deps causes the effect to re-run every render.

**Wrong (reconnects every render because `options` is new each time):**

```tsx
function ChatRoom({ roomId }) {
  const options = { serverUrl, roomId }; // new object each render
  useEffect(() => {
    const conn = createConnection(options);
    conn.connect();
    return () => conn.disconnect();
  }, [options]); // fires every render
}
```

**Correct (only the primitive `roomId` is in deps):**

```tsx
function ChatRoom({ roomId }) {
  useEffect(() => {
    const options = { serverUrl, roomId }; // created inside effect
    const conn = createConnection(options);
    conn.connect();
    return () => conn.disconnect();
  }, [roomId, serverUrl]);
}
```

## Cleanup rule 1 — Always return a cleanup function from subscriptions

Every `addEventListener`, `setInterval`, WebSocket, or observer needs a matching cleanup.

```tsx
useEffect(() => {
  const conn = createConnection(serverUrl, roomId);
  conn.connect();
  return () => conn.disconnect(); // REQUIRED
}, [roomId]);

useEffect(() => {
  function handleScroll() { console.log(window.scrollY); }
  window.addEventListener('scroll', handleScroll);
  return () => window.removeEventListener('scroll', handleScroll); // REQUIRED
}, []);
```

Omitting cleanup causes memory leaks and double-subscription bugs in StrictMode.

## Cleanup rule 2 — Data fetching needs an ignore flag

```tsx
useEffect(() => {
  let ignore = false;

  async function fetchData() {
    const result = await fetchTodos(userId);
    if (!ignore) setTodos(result); // dropped if the effect re-ran
  }

  fetchData();
  return () => { ignore = true; };
}, [userId]);
```

Without the flag, a slow first request completing after a fast second request sets stale data.

## Cleanup rule 3 — Development double-fire is intentional; fix cleanup, not the guard

React remounts in StrictMode to verify cleanup works. If your effect breaks on remount, the cleanup is wrong.

**Wrong (hides the bug):**

```tsx
const didInit = useRef(false);
useEffect(() => {
  if (didInit.current) return;
  didInit.current = true;
  subscribe();
}, []);
```

**Correct (make remounting safe by cleaning up properly):**

```tsx
useEffect(() => {
  const sub = subscribe();
  return () => sub.unsubscribe();
}, []);
```

A ref is per-instance, so it silently re-runs the "one-time" work whenever the component remounts. For app initialization (analytics, auth check) that truly must not repeat, use a module-level flag, which survives remounts:

```tsx
let didInit = false;

function App() {
  useEffect(() => {
    if (didInit) return;
    didInit = true;
    loadDataFromLocalStorage();
    checkAuthToken(); // calling twice may invalidate the token
  }, []);
}
```

If the work doesn't need React at all, run it at module scope instead:

```tsx
if (typeof window !== 'undefined') {
  checkAuthToken();
  loadDataFromLocalStorage();
}
```

## Subscription rule — Use `useSyncExternalStore` for read-only external state

When the effect only mirrors an external value into state, `useSyncExternalStore` does it without the extra state, the extra render, or the SSR mismatch.

**Wrong (effect + state; initial render shows a guessed value):**

```tsx
function useOnlineStatus() {
  const [isOnline, setIsOnline] = useState(true);
  useEffect(() => {
    function update() { setIsOnline(navigator.onLine); }
    window.addEventListener('online', update);
    window.addEventListener('offline', update);
    return () => {
      window.removeEventListener('online', update);
      window.removeEventListener('offline', update);
    };
  }, []);
  return isOnline;
}
```

**Correct:**

```tsx
import { useSyncExternalStore } from 'react';

function subscribe(callback: () => void) {
  window.addEventListener('online', callback);
  window.addEventListener('offline', callback);
  return () => {
    window.removeEventListener('online', callback);
    window.removeEventListener('offline', callback);
  };
}

function useOnlineStatus() {
  return useSyncExternalStore(
    subscribe,
    () => navigator.onLine, // client snapshot
    () => true              // server snapshot (SSR)
  );
}
```

`subscribe` must be defined outside the component or memoized — a new function identity resubscribes on every render. Keep `useEffect` when the subscription has side effects beyond reading a value (sending messages, writing state elsewhere).

## Decision table — which fix to apply

| Symptom | Fix |
|---|---|
| State needs reset when identity prop changes | `key={identityProp}` on the component |
| Child notifies parent via effect | Move notification into the event handler |
| Child pushes fetched data up to the parent | Fetch in the parent, pass down as a prop |
| Multiple effects update shared state in sequence | Merge into one event handler |
| Effect "adjusts" state when a list prop changes | Store the ID, derive the object during render |
| Effect mirrors an external value into state | `useSyncExternalStore` |
| `eslint-disable react-hooks/exhaustive-deps` in codebase | Fix code: functional update or `useEffectEvent` |
| Object/function dep causes effect to re-run every render | Move object/function creation inside the effect |
| No cleanup on event listener or subscription | Return a cleanup function |
| Stale data after fast navigation | `ignore` flag in fetch effect |
| Effect fires twice in dev and breaks | Fix cleanup so remount is idempotent |

## When NOT to apply these patterns

- **Resetting with `key`:** If two logically separate states should survive an identity change, `key` resets both. Verify that ALL state inside the component should reset.
- **Merging handlers:** If two state transitions are genuinely independent and need separate effects (e.g., logging vs. network), keep them separate.

## Common failure modes

| Mistake | Consequence |
|---|---|
| Using an effect to reset state on prop change | Extra render; race condition if cleanup runs first |
| Effect notifying parent after every state change | Double render; parent sees old state on first call |
| Suppressing exhaustive-deps lint rule | Stale closures; silent incorrect behavior |
| Non-primitive dep without moving inside effect | Effect re-runs every render, causes infinite loops |
| No cleanup on WebSocket/EventListener | Memory leak; duplicate event callbacks in StrictMode |
| No ignore flag in async fetch | Stale data race; last-to-complete wins instead of last-to-start |
| Child pushing data up through an effect | Untraceable data flow; parent renders once with `null` |
| Storing a selected object instead of its ID | Selection lost on every list update |
| Inline `subscribe` passed to `useSyncExternalStore` | Resubscribes on every render |
| `useRef` guard for one-time initialization | Work repeats on remount; ref is per-instance |
| Module-level mutable state holding request data | Cross-request data leaks under SSR (flags are fine, data is not) |
