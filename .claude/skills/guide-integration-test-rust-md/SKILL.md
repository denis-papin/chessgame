---
name: guide-integration-test-rust-md
description: Write or fix the Markdown specification files for Rust integration tests. Use when the user wants to author, review, or correct an `integration_test_*.md` (or `unit_test_*.md`) spec under `_ai/<FEATURE>/`, or when they want the MD spec to match the conventions of an existing integration-test `.rs` file. Triggers on phrases like "write the integration test MD", "fix the IT spec", "document the integration tests for Fxxxx".
---

# Integration-test Markdown Guide Lines specs

This skill encodes the conventions for the Markdown files that **specify** Rust
integration tests. These MD files live next to the feature design under
`_ai/<FEATURE-ID>/` and act as the human-readable, traceable contract that the
dedicated integration-test `.rs` file implements.

The MD is a **specification**, not a transcript. It is written *before or
alongside* the `.rs` file, in black-box language, and every assertion it lists
maps to a concrete behavioural outcome and the business rule that outcome
proves.

Before writing a new spec, read any existing exemplars in the repo:
- The sibling `integration_test_*.md` specs under `_ai/<FEATURE>/`.
- Their `.rs` counterparts in the project's integration-test directory.
- Any shared test harness/helper module the tests rely on.

---

## When to use

- Authoring a brand-new `integration_test_<slug>.md` for a feature/fix.
- Correcting an existing spec so it matches the conventions below.
- Re-syncing an MD spec with its `.rs` file after the test code changed
  (test names, TC numbering, seeded fixtures, assertions).

If the user only wants the `.rs` test code, this skill still applies for the
spec; flag that the two must stay in lock-step.

---

## File location & naming

- Path: `_ai/<FEATURE-ID>/integration_test_<slug>.md`
  (e.g. `_ai/F0003-some-feature/integration_test_some_feature.md`).
- Unit-test specs use `unit_test_<slug>.md` / `unit_tests_<slug>.md` in the
  same folder — same structure, but the "System under test" is the internal
  function, not the public API.
- The `<FEATURE-ID>` folder also holds the feature design (`F000X.md`). Keep
  the naming prefix consistent: feature `F0003` → spec id `IT-F0003`.

---

## Required structure

Produce the sections **in this order**:

### 1. YAML front-matter (fenced ```yaml block, not `---`)

Use a fenced ```yaml block at the very top (with a leading blank line),
**not** Jekyll-style `---` fences.

```yaml
id: IT-F0004
title: Integration tests — <short description of the behaviour under test>
type: integration-test
status: draft
target_stream: <BUSINESS-STREAM>
related_feature: F0004      # or related_fix: F0003 for a bug fix
naming_prefix: IT-F0004
language: rust
```

Rules:
- `id` / `naming_prefix` = `IT-` + feature id.
- `type`: `integration-test` (or `unit-test`).
- `status`: starts at `draft`.
- `target_stream`: the business flow under test (e.g. a named feature stream).
- Use `related_fix:` for bug branches (`Bxxxx`/`Fxxxx` fixes), `related_feature:`
  for features.

### 2. `# Coverage goal`

A short prose paragraph stating the **behavioural, end-to-end** intent, with the
key phrase that this is validated **through the public API** (black-box).
Follow it with a bullet list: each bullet is one observable outcome and, in a
dash clause, the rule it proves. Example shape:

> Behavioural end-to-end validation, **through the public API**, that … .
> What is asserted (black-box, no internal-state inspection):
> - <observable outcome> — proves <rule> …;
> - …

### 3. `# System under test`

Name the **real** moving parts explicitly:
- The server(s)/process(es) and the host/port actually used by the harness.
- The client methods or entry points exercised, fully qualified
  (module path + method names).
- State the boundary plainly: **"No mock, no internal-state inspection — the
  test only observes what the public interface returns."**
- When a signature makes the cases reachable, quote it in a ```rust block and
  add a wire-form table for the relevant arguments.

### 4. `# Test cases`

One subsection per case: `## TC-<PREFIX>-NNN — <one-line title>`.
- Number cases `001`, `002`, … The numbering must align with the `.rs` test
  function order (see "TC ↔ test-fn mapping" below).
- Each case uses the **Given / (Input / Filter) / When / Then** skeleton:

  - **Given**: the fixture/seed data and any randomized inputs (note when they
    are randomized "to avoid cross-run collisions on shared state"), and a
    **markdown table** of seeded items where useful:

    | Item | `<field>` value           | Expected     |
    |------|---------------------------|--------------|
    | A    | `<value A>`               | yes          |
    | B    | `<value B>`               | no — reason  |

    Always give the *reason* in the "Expected" cell for the negative rows.

  - **Input** (or **Filter**): the exact input string/payload, in backticks.
    For escaping or parsing features, add the parser-level facts (canonical
    form, unescaped value) under a note, marked "already verified by IT-F000x,
    kept here for traceability".

  - **When**: the exact client call, e.g. the fully-qualified method invocation
    with its arguments.

  - **Then**: a **numbered** list. Each item states the observable outcome
    (which item is returned/excluded, `Ok(_)` / `Err(_)`) **and** the rule it
    proves, written as "proves …". Tie every behaviour back to a business rule
    for traceability. Include the implicit success assertion where relevant
    (e.g. the call must return `Ok(_)`; a 5xx means the request was malformed).

### 5. `# Regression coverage` (optional)

Call out any previously `#[ignore]`d or now-redundant test that this IT
supersedes, with a link, and state it should be removed.

### 6. `# Test file`

- Link to the dedicated `.rs` file with a **relative** link from the MD's
  location (e.g. `[`tests/F0004-some-feature.rs`](../../tests/F0004-some-feature.rs)`).
- State the module name (e.g. `f0004_some_feature_tests`).

---

## TC ↔ test-fn mapping (keep MD and .rs in lock-step)

The `.rs` files follow these conventions — the MD must mirror them:

- One **dedicated file per feature** in the project's integration-test
  directory.
- Tests live in `#[cfg(test)] mod fXXXX_<slug>_tests { … }`.
- Test fn naming: `t<NN>_fXXXX_<short_desc>`, numbered by tens
  (`t10_…`, `t20_…`, `t30_…`). **TC-Fxxxx-001 ↔ t10_…, -002 ↔ t20_…**, etc.
  (A gap like `t110_…` for `TC-…-011` is fine — preserve it in both.)
- Every test returns a `Result<(), …>` and ends with its teardown + `Ok(())`.
- Randomized fixtures via a local helper where cross-run isolation is needed.
- Servers/clients are constructed with the host/port the project actually uses.
- Each test fn carries a header comment block restating the TC's
  Given/When/Then, and every `assert!` has a message that ties the outcome to
  the rule it proves (mirror the MD's "proves …" wording).

When **correcting** an MD, cross-check against the `.rs`:
- Does every `## TC-…` section have a matching test function (and vice versa)?
- Do the seeded items / input strings / assertions in the MD match the code?
- Do the TC numbers line up with the `t<NN>` numbering?
Report any drift explicitly to the user.

---

## Style rules (project-wide)

- Write in **English**, in a precise, black-box, rule-traceable tone.
- Do **not** hard-wrap prose. A Markdown sentence can run as a single long line
  without any CR/LF inside it — let the editor soft-wrap. Only use line breaks
  between paragraphs, list items, and other block elements.
- Use backticks for code, inputs, parsed forms, and identifiers.
- Use relative markdown links (`../../…`) to source files so they resolve from
  the `_ai/<FEATURE>/` folder. Always link the `.rs` file, the relevant source
  when a rule references it, and sibling IT specs for traceability.
- Never claim internal-state inspection or mocks — these tests are strictly
  black-box over the public interface.
- Keep a single source of truth per fact: facts proven by an earlier IT are
  *referenced*, not re-proven ("kept here for traceability").

---

## Workflow

1. Read the feature/fix design (`_ai/<FEATURE>/F000X.md`) to learn the business
   rules and the flow id.
2. Read any sibling exemplar specs for structure.
3. If the `.rs` file already exists, read it and derive TC sections from the
   actual tests; otherwise design the test cases from the rules.
4. Produce the MD in the section order above.
5. Cross-check MD ↔ `.rs` (TC numbering, seeded items, inputs, assertions) and
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
  goal, System under test, Test cases, Test file, …). Never drop or merge a
  section.
- Keep all tables, fenced ```yaml`/```rust blocks, backticked identifiers, and
  relative links exactly as they are.
- Keep every Given / Input / When / Then item and every numbered "proves
  <rule>" clause — that traceability wording is the contract, not filler.
- Keep "kept here for traceability" / "already verified by IT-F00x" notes.
- Do **not** hard-wrap prose; preserve the soft-wrap convention from the style
  rules above.

In short: it tightens how each sentence reads, not what the spec asserts. If the
only way to shorten something is to drop a fact, leave it as-is.
