# ADR-0015 — Post-v0 capability ordering: graph backlog, dependency-aware selection, and the dependency view

## Status
Accepted as sequencing guidance for post-v0 work. Nothing here is built in v0. This
ADR exists to record the *order* these capabilities must come in and the ripples
each one causes, so the dependency between them is not rediscovered mid-build.

## Context
A dependency *view* (intents as a graph, edges showing what blocks what, what is
unblocked, what is on the critical path) is appealing, but it cannot exist on v0's
data and is not useful without execution changes. Untangling why surfaced a chain of
prerequisites that are easy to collapse into one.

Two upgrades are involved, and they are independent:
- **Data model:** the backlog is a linear ordered list today (`- [ ]` items in
  sequence). A dependency view requires intents to *declare relationships* — a graph,
  not a list. This is about how work is *described*.
- **Execution:** v0 runs `--once` (a single pass per invocation). Going beyond that
  to a multi-iteration / unattended loop is about how work is *executed*.

You can have either without the other. A branching backlog could be driven by manual
`--once` calls; a multi-iteration loop could march down a still-linear list. So the
dependency view's hard prerequisite is the graph-shaped backlog, not multi-iteration.

But they are complementary, and the view is mostly pointless without the loop: the
value of "what is unblocked / what can run next / what is parallelizable" only cashes
out when something *consumes* it to make decisions. Under manual `--once` the operator
already knows which item they are running — they chose it. The graph earns its keep
only once the loop is autonomous enough to route by it.

## Decision
Sequence post-v0 work in this order; do not attempt a later item before its
prerequisite lands.

1. **Graph-shaped backlog (root prerequisite).** Replace the linear list with a model
   where an intent can declare its dependencies. This is upstream of both the smart
   loop and the view.
2. **Dependency-aware selection.** Change `run`'s selection from "next unaddressed
   intent" (which assumes a total order) to "next intent whose dependencies are all
   satisfied." Note this selection rule is *meaningless* on a linear list — "next
   unblocked" just means "next in line" — which is why it depends on step 1.
3. **Multi-iteration / unattended loop.** The execution upgrade beyond `--once`
   (multi-pass, `--afk`, `--watch`, `--max-iters`). This is what makes the graph
   backlog worth having, because it is the thing that routes by it automatically.
4. **Dependency view (visualization).** Only now is it both *possible* (the graph
   exists) and *worthwhile* (the loop is using it to route). It is the surface of a
   deeper capability, not a standalone visualization task.

Corollary for the visualization roadmap (ADR-/forthcoming visualization ADR): the
dependency view does NOT belong in the visualization bucket next to the per-run
bundle renderer and the provenance graph. Those are read-only projections of data
v0 already produces. The dependency view is downstream of a data-model and an
execution change and must be sequenced after them.

## Ripples to account for (do not treat the graph backlog as a small tweak)
- **`run` selection logic** changes from total-order to dependency-satisfied (step 2
  above), which is a real change to the core loop, not a data rename.
- **Terminal states gain a new flavor.** v0's "no open intent" assumes a total order:
  backlog exhausted. A graph backlog introduces a distinct stalled state — "no
  *unblocked* intent remaining, but blocked intents exist" — which is
  progress-stalled-pending-something, not backlog-exhausted. This is a new terminal
  condition to define, distinct from the exhausted+100 / exhausted+<100 cases pinned
  for v0.
- **Integrity loop interaction.** The ADR-drift / integrity checks become more
  interesting on a graph backlog and should be revisited when step 1 lands.

## Relationship to other ADRs
- Builds on the v0 terminal-state and selection decisions (the B3/B4-era ADRs,
  ADR-0009/0010): those assume a total order and must be revisited at step 1/2, not
  silently broken.
- Independent of ADR-0013 (sandbox) and ADR-0014 (model heterogeneity), but the
  multi-iteration loop (step 3) is the trigger condition named in ADR-0013 for the
  sandbox upgrade and in ADR-0014 for cost-routing — i.e. step 3 is the inflection
  point where several deferred upgrades become live at once.

## Mental model
- `--once` → multi-iteration is an **execution** upgrade.
- linear list → dependency graph is a **data-model** upgrade.
- The dependency view needs the data-model upgrade and is only useful alongside the
  execution upgrade.

## Consequences
- The post-v0 backlog is reordered: "dependency graph view" is relocated from the
  visualization group to the tail of this capability chain.
- Whoever picks up the graph backlog knows up front that it touches selection,
  terminal states, and the integrity loop — it is not an isolated change.
- This is itself a natural set of dogfood targets once v0 is shipping: the graph
  backlog and dependency-aware selection are bounded, well-specified product lines
  the factory could build on itself.

## Alternatives considered
- **Build the dependency view as a visualization task now.** Rejected: there is
  nothing non-linear to render, and no consumer of the graph to make it worthwhile.
  It would be a line drawn as a graph.
- **Jump straight to multi-iteration on the linear list.** Possible, and may be worth
  doing for unattended runs of sequential work — but it does not unlock the
  dependency view and does not need the graph backlog, so it is a separate decision,
  not this chain.
