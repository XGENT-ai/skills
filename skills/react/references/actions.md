---
name: react-actions
description: Handle form submissions and async mutations with React 19 Actions — useActionState, useFormStatus, and useOptimistic.
license: MIT
last_reviewed: 2026-09-26
---

# React Actions — forms, pending state, and optimistic updates

## When to apply

- A form tracks `loading` / `error` / `data` by hand, with `useState` flags or a reducer with `'loading' | 'success' | 'error'` cases
- `onSubmit` calls `e.preventDefault()` and builds `FormData` manually
- A submit button receives `isPending` through props from the form above it
- An optimistic update is hand-rolled: snapshot the list, add the item, restore the snapshot in `catch`

Actions are React 19 only. For a non-form async update that just needs `isPending`, see [`useTransition`](rendering-performance.md#pattern-10--use-usetransition-over-manual-loading-states).

## What an Action is

An Action is a function React runs inside a Transition: a function passed to `<form action>` or `formAction`, or one called inside `startTransition`. React tracks its pending state, sends thrown errors to the nearest error boundary, and lets `useActionState` and `useOptimistic` hook into it. After a `<form action>` succeeds, React resets the form's uncontrolled fields.

One limitation: state updates after an `await` are not part of the Transition. Wrap them in another `startTransition`:

```tsx
startTransition(async () => {
  const saved = await save(draft)
  startTransition(() => setItems(curr => [...curr, saved]))  // after await: wrap again
})
```

## Pattern 1 — `useActionState` instead of hand-managed loading state

```tsx
const [state, dispatchAction, isPending] = useActionState(reducerAction, initialState)
// reducerAction(previousState, payload) — with <form action>, payload is the FormData
```

**Incorrect (reducer + manual dispatch around the request):**

```tsx
function SignupForm() {
  const [state, dispatch] = useReducer(signupReducer, { loading: false, error: null })

  async function handleSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault()
    dispatch({ type: 'loading' })
    try {
      await signup(new FormData(e.currentTarget))
      dispatch({ type: 'success' })
    } catch (err) {
      dispatch({ type: 'error', error: (err as Error).message })
    }
  }

  return (
    <form onSubmit={handleSubmit}>
      <input name="email" />
      {state.error && <p role="alert">{state.error}</p>}
      <button disabled={state.loading}>Sign up</button>
    </form>
  )
}
```

**Correct:**

```tsx
import { useActionState } from 'react'

type State = { error: string | null }

async function signupAction(prev: State, formData: FormData): Promise<State> {
  const result = await signup(formData.get('email') as string)
  if (!result.ok) return { error: result.message }  // expected failure → state
  return { error: null }
}

function SignupForm() {
  const [state, formAction, isPending] = useActionState(signupAction, { error: null })

  return (
    <form action={formAction}>
      <input name="email" />
      {state.error && <p role="alert">{state.error}</p>}
      <button disabled={isPending}>Sign up</button>
    </form>
  )
}
```

Rules:

- **Return expected errors, throw unexpected ones.** A thrown error cancels all queued actions and shows the nearest error boundary instead of the form. Validation failures and 4xx responses belong in the returned state.
- **Outside a form, dispatch inside `startTransition`.** Called from `onClick` without it, React logs an error and `isPending` doesn't track the call:

  ```tsx
  <button onClick={() => startTransition(() => dispatchAction({ type: 'remove', id }))}>Remove</button>
  ```

- **Calls queue.** Multiple dispatches run in order; each `reducerAction` receives the previous result.
- **The form resets on success.** Uncontrolled fields clear after the action succeeds. To keep what the user typed after an error, return the submitted values in state and render them as `defaultValue`.

## Pattern 2 — `useFormStatus` instead of passing `isPending` down

`useFormStatus` is imported from **`react-dom`**, not `react`. It reports the status of the **parent** `<form>` only, so call it in a component rendered inside the form.

**Incorrect (same component renders the form — `pending` is always `false`):**

```tsx
import { useFormStatus } from 'react-dom'

function CommentForm({ action }: Props) {
  const { pending } = useFormStatus()  // no parent <form> here
  return (
    <form action={action}>
      <textarea name="body" />
      <button disabled={pending}>Post</button>
    </form>
  )
}
```

**Correct (a child of the form reads it — no prop drilling):**

```tsx
import { useFormStatus } from 'react-dom'

function SubmitButton({ children }: { children: ReactNode }) {
  const { pending } = useFormStatus()
  return <button disabled={pending}>{pending ? 'Saving…' : children}</button>
}

function CommentForm({ action }: Props) {
  return (
    <form action={action}>
      <textarea name="body" />
      <SubmitButton>Post</SubmitButton>
    </form>
  )
}
```

It works with a function passed to `<form action>`. A plain `onSubmit` form never reports `pending`.

## Pattern 3 — `useOptimistic` instead of snapshot-and-revert

```tsx
const [optimisticState, setOptimistic] = useOptimistic(value, reducer?)
```

Call the setter inside an Action. When the Transition finishes, the optimistic state converges to whatever `value` is by then. If the Action updated `value`, the confirmed data shows. If it failed and `value` didn't change, the UI shows what it showed before, so there is no manual revert.

**Incorrect (manual copy, manual revert):**

```tsx
function TodoList({ initialTodos }: Props) {
  const [todos, setTodos] = useState(initialTodos)

  async function handleAdd(text: string) {
    const temp = { id: `temp-${Date.now()}`, text }
    const snapshot = todos
    setTodos([...todos, temp])
    try {
      const saved = await api.createTodo(text)
      setTodos(curr => curr.map(t => (t.id === temp.id ? saved : t)))
    } catch {
      setTodos(snapshot)  // stale if anything else changed meanwhile
    }
  }
}
```

**Correct:**

```tsx
import { startTransition, useOptimistic, useState } from 'react'

function TodoList({ initialTodos }: Props) {
  const [todos, setTodos] = useState(initialTodos)  // confirmed state
  const [optimisticTodos, addOptimistic] = useOptimistic(
    todos,
    (current: Todo[], draft: Todo) => [...current, { ...draft, pending: true }]
  )

  async function addTodo(formData: FormData) {
    const text = formData.get('text') as string
    addOptimistic({ id: crypto.randomUUID(), text })  // shows immediately
    const saved = await api.createTodo(text)            // throws → item disappears, error boundary shows
    startTransition(() => setTodos(curr => [...curr, saved]))  // after await: wrap again
  }

  return (
    <>
      <ul>
        {optimisticTodos.map(t => (
          <li key={t.id} style={{ opacity: t.pending ? 0.5 : 1 }}>{t.text}</li>
        ))}
      </ul>
      <form action={addTodo}>
        <input name="text" required />
        <SubmitButton>Add</SubmitButton>
      </form>
    </>
  )
}
```

Generate IDs in the Action and pass them in. The reducer runs during render, so an ID created there changes on every render and remounts the item.

## Finding refactor candidates

```bash
grep -rnE "case ['\"](loading|pending|success|error)['\"]" src/   # reducer-managed request state
grep -rnE "set(Is)?(Loading|Submitting|Pending)\(true\)" src/     # hand-rolled pending flags
grep -rn "preventDefault()" src/ | grep -i submit                  # onSubmit handlers
```

Refactor these after the React 19 upgrade is stable, not as part of it (see [React 19 migration](react19-migration.md#ground-rule--an-upgrade-pr-changes-api-surface-only)).

## Decision table — React 18 pattern → React 19 Action

| React 18 pattern | React 19 replacement |
|---|---|
| `onSubmit` + `preventDefault` + `new FormData(e.target)` | `<form action={fn}>` receives `FormData` |
| `useReducer` with `loading` / `success` / `error` cases | `useActionState` — return the next state |
| Manual `setIsLoading(true/false)` | `isPending` from `useActionState` or `useTransition` |
| `isPending` passed to the submit button as a prop | `useFormStatus()` in the button (from `react-dom`) |
| Snapshot, update, restore in `catch` | `useOptimistic` — converges automatically |
| Error shown via `catch` + state | Return expected errors as state; error boundary for the rest |

## When NOT to apply

- **React 18 codebases:** none of these APIs exist.
- **Forms owned by a form library** (react-hook-form, Formik) with field-level validation as you type: don't mix two sources of truth for pending and error state.
- **Mutations that need retries, cancellation, or cache invalidation:** a data library's mutation API (e.g. React Query `useMutation`) already handles them.
- **During the React 19 upgrade PR itself:** adopt Actions in a follow-up.

## Common failure modes

| Mistake | Consequence |
|---|---|
| `import { useFormStatus } from 'react'` | Import fails — it lives in `react-dom` |
| `useFormStatus` in the component that renders the `<form>` | `pending` is always `false` |
| Throwing for validation errors in a `useActionState` action | Queued actions cancel; the error boundary replaces the form |
| `dispatchAction` from `onClick` without `startTransition` | Dev error; `isPending` doesn't update |
| `useOptimistic(initialProp)` where the prop never updates | Optimistic item vanishes when the Transition ends |
| Optimistic setter called outside an Action | Warning: "An optimistic state update occurred outside a Transition or Action" |
| State update after `await` without `startTransition` | The update escapes the Transition |
| Random ID generated inside the `useOptimistic` reducer | Key changes every render; the item remounts |
| Relying on field values after a successful submit | Uncontrolled fields reset; return values in state if they must persist |
