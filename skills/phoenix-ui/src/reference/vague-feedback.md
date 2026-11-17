# Vague feedback

Read this when the user judges an existing interface with a word that names no change: "make it more premium", "not good-looking enough", "simpler", "再高级一点", "不够好看", "简洁些". The word is a direction of travel, not a decision. Turn it into observable variables before touching the surface.

## Translate first

Read the surface first if you need to. Then, before any Edit or Write to it, reply with the translation and stop there:

```text
"<the user's word>", as changes that can be checked:
- <variable to change> → <expected change>
- <variable to change> → <expected change>

Reply to go ahead, or say which lines to change.
```

One line per variable. A variable is something a reader can check in the render or the computed styles: `hue count → from five to two plus one accent`, `section separation → rules removed, spacing step raised from 24px to 40px`.

That reply ends the turn. Nothing on the surface changes until the user answers; they may confirm the list, strike lines, or add to it, and you edit to the list as it then stands. In an unattended run nobody can answer, so the list is the delivery and the surface stays as it is.

After the answer, make exactly those changes, and in the delivery go back through the same list item by item and say what was actually changed.

When the judgment also matches a refine command (calmer maps to `{{command_prefix}}phoenix-ui quieter`, more daring to `bolder`, simpler to `distill`), enter that command once the user has answered and follow its reference; the list is what the command executes.

## What each word can mean

| Word | Translate into |
|---|---|
| premium (高级) | First name which kind of premium this product means; then the typeface contrast, how sections are separated, the spacing scale, and the number of hues |
| refined (精致) | Tolerances: letter-spacing, the grid everything aligns to in px, how many shadow levels, the ceiling on hues |
| restrained (克制) | Budgets: a saturation budget such as the share of the surface the accent may cover, a ceiling on motion amplitude, a ceiling on hues |
| epic (史诗) | Scale: display type size, the area the lead image owns, the light-to-dark contrast |
| simple (简洁) | The density step, plus what is removed and why |

Other words get the same treatment: name the variables the word would move on this surface. Take the values from PRODUCT.md, DESIGN.md, and the committed visual world, not from a generic idea of the word.

## Words that do not translate

An adjective that yields no observable variable is not a reason for a change. Leave it out of the plan and out of the delivery: "more premium" never appears as the justification for an edit, only the variables do.

## When the meaning is unclear

When the word could mean two different things on this surface, the same reply offers both as two short lists, each stated as its variables, and asks which one; use the structured question tool where the host has one. That is the only question. Or build two comparable variants instead of asking: element-level in [live.md](live.md), whole-page on the comparison page ([compare.md](compare.md)). Do not follow the user's answer with another round of questions, and never answer vague feedback by swapping in a different style at random.

## Boundaries

- During a direction round, "bolder" and "safer" are the round's re-roll registers ([new-work.md](new-work.md)); they stay there and do not come through this reference.
- A user who asks for a different direction goes through the existing direction flow, not a translation.
