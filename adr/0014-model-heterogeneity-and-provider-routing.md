# ADR-0014 — Model heterogeneity and provider routing

## Status
Accepted for Tier 0 and Tier 1 (below); Tier 2 explicitly deferred as a recorded
fork. This builds on ADR-0001 (agent and judge are swappable, injectable
interfaces) — it does not change that seam, it sets policy for how to use it.

## Context
ADR-0001 made the agent and the judge separate swappable providers, but said
nothing about *which* model belongs in *which* role. As real providers get added
(the big-player coding agents, plus OpenAI-compatible endpoints including a local
Ollama server), a policy is needed — both to spend capability where it matters and
to preserve an integrity property that is easy to lose by accident.

There are two provider archetypes, with different powers:
- **Harness providers** (Claude Code, Codex, Gemini CLI): a battle-tested internal
  plan→implement→review loop for free, but a black box — one logical agent, no
  role-splitting inside it.
- **Raw-model providers** (Anthropic/OpenAI API directly, or an OpenAI-compatible
  endpoint such as Ollama): allow assigning a model per role, because the caller
  owns the decomposition — but that means rebuilding scaffolding the harness gave
  you (the ADR-0004 trap).

## Decision

### Role-to-capability policy
Spend capability where a mistake is expensive and irreversible; save it where a
mistake is cheap and visible.

| Role | Capability need | What to run there |
|---|---|---|
| Implement | Highest | Frontier model — do not cheap out |
| Plan | High (errors cascade downstream) | Frontier or strong mid-tier |
| Judge | Moderate, but integrity-critical | A *different model family* than the implementer; local/cheap acceptable only if calibrated (see below) |
| Summarize / triage / no-op checks | Low | Local/cheap (Ollama) — ideal |

### Tier 0 — Decouple the judge (adopt now)
Point the judge provider at a different model than the implementer. Two payoffs:
- **Anti-collusion (primary).** This is on-thesis with ADR-0002: the mind that
  writes the code should not be the mind that grades it. A judge running the *same*
  model as the implementer shares its blind spots and can miss the very flaw a
  shared misconception produced. A different model *family* judging reduces this
  correlated failure.
- **Cost / privacy.** A cheaper or local judge for high-volume validation.

**Guard — a weak judge is worse than no judge.** A judge that rubber-stamps
manufactures false greens, the exact failure the whole system exists to prevent. So
"different family for independence" is good; "much weaker model to save money" is
dangerous. If a cheap/local judge is used, calibrate it periodically against a
frontier judge on a fixed set of known-verdict scenarios, and trust it only while it
agrees. (The judge that judges the judge.)

### Tier 1 — Multiple harness agents, routable (adopt when provider choice is wanted)
Register Claude Code, Codex, Gemini CLI, etc. as N implementations behind the
ADR-0001 interface, with the factory's outer loop choosing which. This is the
"factory routes work between stations" lesson — no role-splitting inside a harness,
no inner loop to rebuild, fully within the delegate-the-harness discipline (ADR-0004).

### Ollama / local providers
A local provider is just an "OpenAI-compatible endpoint" provider pointed at
localhost; one implementation covers Ollama and any other compatible endpoint. Be
realistic about capability: local models are weak at autonomous *implementation*
against held-out scenarios (expect mostly ESCALATE), but a strong fit for the
cheap, high-volume, privacy-sensitive roles — calibrated judging-at-volume,
summarization, triage, no-op checks.

### Cost framing
Role-routing is also the economic lever behind the dark-factory "apply more tokens"
mantra: frontier only where it must be (implement, maybe plan), local/cheap
everywhere else (judging-at-volume, summarize, triage). This is how a solo dev makes
high-token-volume operation affordable.

## Tier 2 — Per-role models within implementation (DEFERRED FORK — not taken)
Assigning different models to planning vs implementation *inside* the implement step
(plan with A, implement with B). This is powerful and unlocks full local/cost
routing, but it requires the caller to own the plan→implement decomposition and call
raw models per step — i.e. rebuilding the inner loop a harness provides, which is the
ADR-0004 ("don't build a worse Claude Code") boundary.

Recorded here, deliberately not-taken, so a future session does not wire it up by
reflex and silently re-own the scaffolding we chose to delegate.

- **Tension:** directly collides with ADR-0004.
- **Trigger to revisit:** only if per-role routing economics demonstrably pay (the
  routing savings beat the cost of owning and maintaining the decomposition).
- **If taken:** it earns its own ADR that confronts the ADR-0004 boundary head-on,
  and it should be built *by* the factory once it is dogfooding, not by hand.

## Consequences
- No architectural change for Tier 0/1 — they use the ADR-0001 seam as designed,
  plus the role→capability policy and the judge-independence + calibration rules
  recorded above.
- The judge's model becomes a first-class configuration choice with a safety
  rationale, not an afterthought. A future change that points the judge at the same
  model as the implementer "to keep it simple" is a regression against this ADR, not
  a simplification.
- A sandboxed S006 (ADR-0013) and an independent-family judge are complementary
  integrity layers: one stops the agent reaching the holdout, the other stops a
  same-mind judge sharing the implementer's blind spots.

## Alternatives considered
- **Fold into ADR-0001.** Rejected: the role→capability policy and the
  anti-collusion/calibration rules are a distinct decision with their own safety
  dimension, substantial enough to stand alone.
- **One model for everything (no heterogeneity).** Simpler, but forfeits
  anti-collusion and the cost lever, and couples judge integrity to implementer
  capability.
- **Adopt Tier 2 now.** Rejected for the ADR-0004 reasons above; recorded as a
  deferred fork rather than silently foreclosed.
