# ADR-0016 — Human-in-the-loop control plane: escalation hand-off via async notifications and a verdict store

## Status
Accepted as the design for escalation handling under unattended operation. Not built
in v0. Sits on top of the multi-iteration loop and therefore behind that gate. The
lightweight async design below is v1; a live control plane is recorded as a deferred
heavier alternative.

## Context
v0's terminal states define what ESCALATE *means* (the machinery ran, the work
genuinely doesn't pass, retrying won't help, a human must look) but not what
*happens* when it fires. Under `--once`, nothing needs to happen: a human is already
present and reads the bundle. The moment the loop runs unattended (`--afk`,
multi-iteration), ESCALATE stops being a label on a result the operator is looking at
and becomes an *event that needs a destination* — a hand-off from the loop to a human
who may be away from the desk.

The intended deployment is the loop running on a home server (a Beelink), with the
operator adjudicating from their phone. That deployment makes one architectural
choice unambiguous: the channel must be **asynchronous**. A live connection assumes
the daemon is up and reachable the instant the operator taps — exactly the assumption
that breaks on a headless home box behind reboots, network blips, cellular, and
Tailscale reconnects. A queue-based design decouples the loop and the human in time:
the loop drops a message and continues; the human's reply lands whenever it lands; the
loop reconciles on its next tick. Nothing has to be co-present.

## Decision

### Escalation hand-off model
When the loop reaches ESCALATE under unattended operation it:
1. **Sets the intent aside** per the routing rule (below) and continues with what is
   still workable, rather than halting the whole backlog on one stuck item.
2. **Writes a pending-escalation record** to the verdict store (project, intent, why
   it escalated, satisfaction, link to the evidence bundle).
3. **Fires an outbound notification** to the operator's phone with tap-able actions.

### The verdict store (inbound spine)
A store, resident on the server next to the registry, that is the single source of
truth for "what the human has decided." It starts dead-simple — a directory of JSON
records (`pending/<id>.json`, `verdicts/<id>.json`) or a small SQLite — consistent
with the existing file-based state (registry, evidence bundles). It is another
projection of on-disk truth, which is why it builds on the existing model rather than
introducing a new kind of thing.

**Critical discipline:** the verdict store is state the loop *reads at a safe point*,
never something that reaches into a running pass. A phone tap appends a verdict; the
loop consumes pending verdicts *between iterations*, not mid-implementation. A verdict
that arrives while the loop is mid-pass simply waits for the next tick. This keeps the
control plane async all the way down — even human input is consumed at a defined
boundary — so there are no races and no "what happens if I tap while it's running."

### Verdict vocabulary (v1 — keep it small)
Three verdicts, each a tap:
- **`skip`** — set this intent aside, keep working what is unblocked.
- **`retry`** — I fixed something out of band; try the intent again.
- **`pause`** — stop the loop; I am coming to the desk.

The phone is for **triage and tap-sized adjudication**, not deep work. Resolutions
that are real work — rewriting a scenario, correcting the spec, editing code — are not
verdicts. They are handled by `pause` + desk work, where the fix itself unblocks
progress. The verdict vocabulary must **not** grow to cover deep fixes; that is the
bloat trap. (A likely good side effect: this pressures the agent to emit clean
NEEDS_DECISION questions — which are phone-native, already being specific answerable
choices — rather than bare ESCALATEs.)

### Notifier as a swappable provider
Outbound notification is a `Notifier` behind an interface, same pattern as the
agent/judge providers. v1 implementations target existing push services (ntfy,
Pushover, Telegram, generic webhook) — no custom mobile app. Their action-button
support is what renders the `skip`/`retry`/`pause` taps.

### Network posture (home-server specific)
Outbound is trivial (HTTPS to the push service). The inbound webhook — what the
notification's action buttons hit to write a verdict — lives on the server and is
reachable over the private Tailscale network, with **no public ingress**. This is
simpler and safer than exposing anything publicly.

## Consequences
- ESCALATE changes from a terminus into a hand-off: loop → human → loop. v0 has only
  the first arrow; this adds the round-trip and is the reason `factory resume` is on
  the deferred command list.
- The control plane is entirely file-based and private-network-resident; nothing is
  always-on in the "service you babysit" sense, and nothing is publicly exposed.
- Most of this is a bounded product line the factory could build on itself once the
  loop exists.

## Hard dependencies (build order)
1. **Exit-code / terminal-state plumbing must be correct first (ADR-0012).** The
   notifier decides whether to ping based on the terminal state; a notifier that
   cannot reliably tell ESCALATE from success is useless. (The v0 bug where failure
   terminal states exit 0 is load-bearing here — fixed under ADR-0012.)
2. **The multi-iteration loop must exist.** There is no "set aside and continue"
   without a loop that continues, and no "next tick" on which to reconcile verdicts.
   This control plane sits on top of the loop (the convergence gate), not before it.

## Coupling to the graph backlog (ADR-0015)
The `skip` verdict's meaning depends on the backlog shape. On a linear backlog, `skip`
= "move to the next intent." On the graph backlog, a skipped intent strands its
dependents, so `skip` must mean "set aside this intent *and everything blocked by it*,
work the rest." This is the same boundary where the "no unblocked intent remaining"
terminal state appears (ADR-0015). The control plane can be built on the linear
backlog first (`skip` = next), but `skip`'s semantics change the moment dependencies
exist. Recorded so it is not a surprise.

## Verify before building
Confirm the chosen push service's action-button URLs can reach a Tailscale-internal
address, or whether a small always-listening webhook on the server (reachable over the
tailnet) is needed for the service to call. This is the one mechanical detail to check
rather than assume.

## Alternatives considered
- **Live control plane (daemon the phone connects to in real time; Flutter/WebRTC).**
  More capable — streaming "what's the agent doing now" — but a much larger build, it
  makes the factory a service to operate, it assumes co-presence that a home server
  breaks, and live "watch the agent" is itself premature until the loop runs long
  enough to watch. Deferred as a heavier future alternative, taken only if live
  operation proves necessary.
- **Halt the whole loop on ESCALATE.** Rejected: one stuck intent would freeze a
  backlog full of otherwise-workable items. Set-aside-and-continue is the default,
  with `pause` available as an explicit human verdict when stopping is wanted.
- **No notification — bundle on disk only.** That is v0 behavior; it is exactly what
  fails unattended, because a file on disk is not a hand-off.
