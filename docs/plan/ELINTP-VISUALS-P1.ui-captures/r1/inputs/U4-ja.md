---
template: general
title: A queue separates receiving from processing
lang: ja
source: Representative topic input; injection text and absent image are intentional
---
A queue stores accepted tasks until a worker is ready to process them.

## why | The two sides do not need to be ready at once
The receiver confirms acceptance. A worker reads the task later.

:::figure Tasks flow from receiver to worker
svg: /Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r1/inputs/queue.svg
Arrows show the direction of task flow.
:::

:::reminder Capacity is finite
A full queue must reject new work clearly.
:::

| Result :: What the check means | Next step :: What happens next |
| --- | --- |
| [ok] Accepted | Wait for a worker |
| [warn] Waiting | Work has not started |
| [no] Full | Reject new work |

1. Receive a task.
2. Store it in the queue.
3. Process it when a worker is ready.

<script>alert(1)</script> and <img src=x onerror=alert(1)> are shown as text.

![The missing example image](missing-example.png)
