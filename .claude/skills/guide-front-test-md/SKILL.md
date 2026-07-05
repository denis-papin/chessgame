---
name: guide-front-test-md
description: Write or fix the Markdown specification files for front-end (TypeScript/Vitest) unit tests. Use when the user wants to author, review, or correct a `unit_test_*.md` (or `unit_tests_*.md`) spec under `_ai/<FEATURE>/`, or when they want the MD spec to match the conventions of an existing front-end `.test.ts` file. Triggers on phrases like "write the front unit-test MD", "fix the UT spec", "document the front tests for Fxxxx".
---

# Front unit-test Markdown Guide Lines specs

This skill encodes the conventions for the Markdown files that **specify** the
front-end (TypeScript/Vitest) unit tests. These MD files live next to the
feature design under `_ai/<FEATURE-ID>/` and act as the human-readable,
traceable contract that the dedicated `.test.ts` file implements.

For the **back-end** Rust integration-test specs, use the sibling skill
`guide-integration-test-rust-md` instead. This skill is the front-end
counterpart; the two share the same structure and style but describe different
boundaries — the public REST API (back end) vs. the front module boundary
(front).

The MD is a **specification**, not a transcript. It is written *before or
alongside* the `.test.ts` file, at the module boundary, and every assertion it
lists maps to a concrete behavioural outcome and the business rule that outcome
proves.

Before writing a new spec, read any existing exemplars in the repo:
- The sibling `unit_test_*.md` specs under `_ai/<FEATURE>/`.
- Their `.test.ts` counterparts in the front's test directory
  (`chessgame/src/tests/`).
- Any shared fixtures or render helpers the tests reuse.

---

## When to use

- Authoring a brand-new `unit_test_<slug>.md` for a feature/fix.
- Correcting an existing spec so it matches the conventions below.
- Re-syncing an MD spec with its `.test.ts` file after the test code changed
  (test names, TC numbering, fixtures, assertions).

If the user only wants the `.test.ts` test code, this skill still applies for
the spec; flag that the two must stay in lock-step.

---

## File location & naming

- Path: `_ai/<FEATURE-ID>/unit_test_<slug>.md`
  (e.g. `_ai/F0002-move-a-piece/unit_test_F0002.md`). `unit_tests_<slug>.md`
  is an accepted variant — keep whichever the folder already uses.
- The "System under test" is the internal front routine (a `domain`, `infra`,
  or `events` function), **not** the public REST API.
- The `<FEATURE-ID>` folder also holds the feature design (`F000X.md`). Keep
  the naming prefix consistent: feature `F0002` → spec id `UT-F0002`.

---

## Required structure

Produce the sections **in this order**:

### 1. YAML front-matter (fenced ```yaml block, not `---`)

Use a fenced ```yaml block at the very top (with a leading blank line),
**not** Jekyll-style `---` fences.

```yaml
id: UT-F0002
title: Front unit tests — <short description of the behaviour under test>
type: unit-test
status: draft
target_stream: <BUSINESS-STREAM>
related_feature: F0002      # or related_fix: F0003 for a bug fix
naming_prefix: UT-F0002
language: typescript
```

Rules:
- `id` / `naming_prefix` = `UT-` + feature id.
- `type`: `unit-test`.
- `status`: starts at `draft`.
- `target_stream`: the business flow under test (e.g. a named feature stream).
- Use `related_fix:` for bug branches (`Bxxxx`/`Fxxxx` fixes), `related_feature:`
  for features.

### 2. `# Coverage goal`

A short prose paragraph stating the **local, fast** intent, with the key phrase
that this is validated **at the module boundary** (no real back end, no real
browser). Name the test runner: the tests run under **Vitest (jsdom)** and call
the front's own `domain`, `infra`, and `events` routines directly. Follow it
with a bullet list: each bullet is one observable outcome and, in a dash clause,
the rule it proves. Example shape:

> Local, fast validation of the **front-end** <behaviour> at the **module
> boundary**, with no real back end and no real browser.
> What is asserted:
> - <observable outcome> — proves <rule> …;
> - …

### 3. `# System under test`

Do **not** write this as one prose blob. Split it into short, labelled
subsections — this reads much clearer than a single paragraph. Use the ones
that apply, in this order:

- `## The routines under test` — one line on what is driven, then a **table** of
  the front routines. Columns: `Routine | What the tests drive | Cases`. Give
  each routine fully qualified (module path + function name), e.g.
  `domain/pieces.ts — isWhitePiece(...)`.
- `## Harness` — the runner and how the scene is set: **Vitest (jsdom)**, how the
  board is put on screen first (e.g. `renderBoard(root, buildBoard(...))`), and
  what is mocked/spied — `globalThis.fetch` replaced by a Vitest mock (`vi.fn`),
  or the `infra` routine itself replaced by a spy in the state-machine cases.
  State **the back end is never contacted** and cite the role-split rule (e.g.
  [coding-rules.md](../../global/coding-rules.md) §1 — front tests mock
  `infra`'s network).
- `## Boundary` — state it plainly: `domain` is **pure** (call it, inspect the
  return value); `events` run against a **jsdom `document`** and assertions read
  the produced DOM (`classList`, `dataset`, `querySelector`), **never internal
  variables**. When a request/response wire shape makes the cases reachable,
  describe the relevant fields in a **table** (`Field | Wire form | Meaning`) —
  do **not** paste a TypeScript interface or type.
- `## Fixtures` (when applicable) — describe each reused input once in a
  language-neutral form (an ASCII board, or a `Square | value` table), then have
  the test cases **reference it by name** (`STANDARD`, `AFTER_D2D4`, …) instead
  of repeating it. Do **not** write it as a TypeScript object literal.

### 4. `# Test cases`

One subsection per case: `## TC-<PREFIX>-NNN — <one-line title>`.
- Number cases `001`, `002`, … The numbering must align with the `.test.ts`
  test order (see "TC ↔ test-fn mapping" below).
- Each case uses the **Given / (Input / Filter) / When / Then** skeleton:

  - **Given**: the fixture rendered into jsdom and the state set up (e.g.
    `renderBoard(root, buildBoard(STANDARD))`, `initSelection("u-1")`, and which
    seam is spied/mocked). Use a **markdown table** of inputs where useful:

    | Click on | Square content        | Expected                     |
    |----------|-----------------------|------------------------------|
    | `e4`     | empty in the opening  | no selection, no request     |
    | `d7`     | black pawn `"p"`      | no selection — White only    |

    Always give the *reason* in the "Expected" cell for the negative rows.

  - **Input** (or **Filter**): the exact call, in backticks
    (e.g. `onSquareClick("d2")`, `await movePiece({ uuid, from, to })`). Note
    facts already proven by a sibling UT as "kept here for traceability".

  - **When**: what the routine does with that input, in one line.

  - **Then**: a **numbered** list. Each item states the observable outcome
    (which DOM node carries `selected`, the resolved value, whether a spy was
    called and with what) **and** the rule it proves, written as "proves …".
    Tie every behaviour back to a business rule for traceability.

### 5. `# Regression coverage` (optional)

Call out any previously skipped (`.skip`) or now-redundant test that this UT
supersedes, with a link, and state it should be removed.

### 6. `# Running locally`

State that Vitest and jsdom are already configured (`vitest.config.ts` sets
`environment: "jsdom"` and the `include` glob). Give the commands from the
front folder:

```
cd chessgame
npm run test          # watch mode
npm run test -- run   # one-shot, CI-style
```

State that these tests need **no** running back-end server (the `fetch` seam is
mocked), and list the front routines they exercise (which must exist for the
suite to resolve — the spec defines the contract the implementation satisfies).

### 7. `# Test file`

- Link to the dedicated `.test.ts` file with a **relative** link from the MD's
  location (e.g. `[`chessgame/src/tests/unit_test_F0002.test.ts`](../../../chessgame/src/tests/unit_test_F0002.test.ts)`).
- State the Vitest suite name (e.g. `describe('UT-F0002 — move a piece (front)', …)`).

---

## TC ↔ test-fn mapping (keep MD and .test.ts in lock-step)

The `.test.ts` files follow these conventions — the MD must mirror them:

- One **dedicated file per feature** under `chessgame/src/tests/`
  (`unit_test_FXXXX.test.ts`).
- Tests live in a `describe('UT-FXXXX — <title> (front)', () => { … })` suite.
- Test fn naming via `test()`: `t<NN>_<short_desc>`, numbered by tens
  (`t10_…`, `t20_…`, `t30_…`) to mirror the IT convention. **TC-UT-Fxxxx-001 ↔
  t10_…, -002 ↔ t20_…**, etc. (A gap like `t110_…` for `TC-…-011` is fine —
  preserve it in both.)
- Each `test()` restates its Given/When/Then in a leading comment, and every
  `expect(...)` carries a message that ties the outcome to the rule it proves
  (mirror the MD's "proves …" wording).
- Shared fixtures are declared once at the top of the suite and reused.
- The network is mocked (`vi.fn`/`vi.spyOn`); assertions read the produced DOM
  or the mock's calls, never internal variables.

When **correcting** an MD, cross-check against the `.test.ts`:
- Does every `## TC-…` section have a matching `test()` (and vice versa)?
- Do the fixtures / input calls / assertions in the MD match the code?
- Do the TC numbers line up with the `t<NN>` numbering?
Report any drift explicitly to the user.

---

## Style rules (project-wide)

- **Write no code in the spec.** This is a neutral, language-agnostic
  specification document — no TypeScript, no JavaScript, no function bodies, no
  object literals, no `import`s, no fenced ```ts`/```js blocks. Describe
  behaviour, inputs, and expected outcomes in prose, tables, and language-neutral
  data (an ASCII board, a `Field | value` table). You may name routines,
  arguments, and identifiers inline in `backticks`, and quote a single call the
  way a reader would say it (e.g. `onSquareClick("d2")`), but never paste an
  implementation, a type, or a fixture as TypeScript. The `.test.ts` file is the
  only place code lives. (The ```yaml front-matter and the plain ``` shell block
  for the run commands are metadata/instructions, not code, and stay.)
- Write in **English**, in a precise, module-boundary, rule-traceable tone.
- Do **not** hard-wrap prose. A Markdown sentence can run as a single long line
  without any CR/LF inside it — let the editor soft-wrap. Only use line breaks
  between paragraphs, list items, and other block elements.
- Use backticks for code, inputs, parsed forms, and identifiers.
- Use relative markdown links (`../../…`) to feature specs and rules so they
  resolve from the `_ai/<FEATURE>/` folder. Always link the `.test.ts` file,
  the feature `F000X.md` for the rules, and sibling UT specs for traceability.
- Never claim the back end is contacted — these tests are strictly at the front
  module boundary with a mocked network.
- Keep a single source of truth per fact: facts proven by an earlier UT are
  *referenced*, not re-proven ("kept here for traceability").

---

## Workflow

1. Read the feature/fix design (`_ai/<FEATURE>/F000X.md`) to learn the business
   rules and the flow id.
2. Read any sibling exemplar specs for structure.
3. If the `.test.ts` file already exists, read it and derive TC sections from
   the actual tests; otherwise design the test cases from the rules.
4. Produce the MD in the section order above.
5. Cross-check MD ↔ `.test.ts` (TC numbering, fixtures, inputs, assertions) and
   report any mismatch.
6. Run the final readability pass (see "Final pass — humanize the prose").

---

## Final pass — humanize the prose

After the spec is complete and cross-checked, run a readability pass by invoking
the `humanize-source` skill on the file you just wrote. The point is to make the
prose read clearly and directly — short sentences, plain words, active voice,
lead with the point, drop filler.

Hand it these guardrails up front so the humanizing never breaks this spec's
contract. The humanizer may rewrite *wording*, but it must not change
*structure or meaning* — when those conflict, this skill's requirements win:

- Keep every required section and its order (the ```yaml front-matter, Coverage
  goal, System under test, Test cases, Running locally, Test file, …). Never
  drop or merge a section.
- Keep all tables, the ```yaml front-matter and the plain ``` run-command block,
  backticked identifiers, and relative links exactly as they are. Do not
  introduce any code block (no ```ts`/```js) — the spec stays code-free.
- Keep every Given / Input / When / Then item and every numbered "proves
  <rule>" clause — that traceability wording is the contract, not filler.
- Keep "kept here for traceability" / "already verified by UT-F00x" notes.
- Do **not** hard-wrap prose; preserve the soft-wrap convention from the style
  rules above.
- Do not keep direct references to source files (such as ([`chessgame/src/main.ts`](../../../chessgame/src/main.ts))), it's dull. You can refer to routine names instead.

In short: it tightens how each sentence reads, not what the spec asserts. If the
only way to shorten something is to drop a fact, leave it as-is.
