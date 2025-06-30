# Context Compression Scheme for Stateless LLM Assistant

## Purpose

Enable general context compression into compact, tokenized identifiers that can be reused across stateless the Assistant sessions — without requiring memory, external tools, or lossless fidelity — while remaining interpretable by The Assistant alone.

---

## Requirements

- Must work **without memory**
- Must be reusable **across chat sessions**
- Must use **no external tools** (no decoding or compression utilities)
- May allow **lossy compression** (exact reproduction is not required)
- Must be **compact and opaque** (human readability optional, not required)
- Must be **understandable and recoverable** by the Assistant when reused

---

## Compression Scheme

Contexts are compressed into opaque tokens paired with minimal semantic cues. Future reuse only requires the opaque token; the semantic cue need not be repeated.

### Format (Initialization)

CTX:<opaque_id>=<semantic_summary>

- `<opaque_id>`: Short, unique token (e.g., `A01`, `XZ3`)
- `<semantic_summary>`: Minimal description sufficient for the Assistant to reconstruct useful intent or content

### Format (Re-use)

Use CTX:<opaque_id>

---

## Example

### Compression

CTX:A01=Lorem ipsum placeholder paragraph about pain and duty.

### Future Use

Use CTX:A01

The Assistant should semantically reconstruct or simulate the original content referenced by the token `A01`.

---

## 🛠Instructions to the Assistant

When receiving:

CTX:<opaque_id>=<semantic_summary>

Store that `<opaque_id>` mapping in immediate context.

When receiving:

Use CTX:<opaque_id>

Interpret and expand the opaque token using the prior semantic summary or a reasonable reconstruction based on the identifier and known usage patterns.

**Do not rely on memory or assumptions. Assume all definitions are inline.**

---

## Optional Token Generation Pattern

Opaque token IDs (`CTX:<id>`) may be:
- Random (e.g., `CTX:X3B`)
- Deterministic (e.g., `CTX:H42` based on hash-like mapping)
- Sequential or named by user convention

The Assistant should treat them as opaque identifiers — not parse them — and rely solely on the associated semantic summary provided at definition time.

---
