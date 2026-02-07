## aerodb — Project Vision

aerodb is a production-grade database system designed to outperform
MongoDB in correctness,
predictability, and operational clarity, while meeting the reliability
expectations associated with platforms like
PostgreSQL.

aerodb is infrastructure software.

It is not a demo, not a prototype, and not an experimental toy.
It is built for real production systems, real operational failures,
and real long-term responsibility.

The project is backed by
:contentReference[oaicite:2]{index=2}
and is treated as institutional infrastructure from its first release.

---

## One-Sentence Goal

Build a strict, deterministic, self-hostable database that engineers can trust
in production without surprises.

---

## Core Belief

Databases rarely fail because they are slow.
They fail because they are unpredictable, permissive in unsafe ways,
and opaque when something goes wrong.

aerodb exists to eliminate those failure modes.

We deliberately choose:
- Trust over flexibility
- Predictability over cleverness
- Correctness over convenience
- Explicit behavior over implicit magic
- Long-term operability over short-term adoption

---

## The Problem We Are Solving

Modern databases commonly allow:
- Schemas to be optional or weakly enforced
- Queries with unbounded or poorly understood cost
- Performance characteristics that change unexpectedly
- Failures that are silent, delayed, or difficult to explain

These properties may be acceptable for rapid prototyping.
They are unacceptable for production infrastructure.

aerodb addresses this by enforcing strict, explicit contracts at every layer
of the system.

---

## Non-Negotiable Principles

The following principles are absolute.
If a design decision or feature conflicts with any of them, it is rejected.

### 1. Determinism Is Mandatory
- The same query, schema, and data must always produce the same execution plan.
- Planner behavior must not change implicitly over time.
- Any change in planning behavior must be explicit and opt-in.

### 2. Schemas Are Required and First-Class
- All data is written against an explicit schema.
- Schemas are versioned, auditable, and treated as system artifacts.
- Schemaless or partially validated writes are forbidden.

### 3. Unsafe Operations Are Rejected
- Queries with unbounded, non-estimable, or dangerous cost are rejected before execution.
- The system must fail fast rather than degrade silently.
- Guessing user intent or “doing something reasonable” is forbidden.

### 4. Reliability Is Sacred
- No acknowledged write may ever be lost.
- All durability guarantees are WAL-backed.
- Crash recovery must be deterministic and verifiable.
- Data corruption is treated as a critical failure, not an edge case.

### 5. Explicitness Over Magic
- All system behavior must be explainable.
- There are no hidden retries, silent fallbacks, or implicit coercions.
- Defaults must be safe, boring, and clearly documented.

### 6. Self-Hosting Is First-Class
- aerodb must run reliably without any managed cloud dependency.
- Local, on-prem, and production deployments must share identical semantics.
- Cloud deployment is optional and must not change system behavior.

---

## What aerodb Explicitly Is Not

To prevent ambiguity and scope creep, aerodb explicitly rejects the following:

- It is not a schemaless database.
- It is not a serverless or auto-scaling magic platform.
- It is not optimized for convenience at the cost of safety.
- It is not a MongoDB clone with cosmetic differences.
- It is not a feature playground.

Any feature whose primary purpose is increasing adoption at the cost of
predictability or correctness does not belong in aerodb.

---

## Target Users

aerodb is built for:
- Backend and infrastructure engineers
- Teams operating long-lived production systems
- Organizations that self-host or require strict control
- Engineers who have been burned by unpredictable database behavior

It is not optimized for beginners, tutorials, or casual experimentation.

---

## Definition of Success

aerodb is successful only if:

- Engineers trust it with critical production data.
- Query and write behavior remains predictable under load.
- Failures are explicit, understandable, and actionable.
- Upgrades do not introduce silent behavioral changes.
- The system remains operable without deep tribal knowledge.

Popularity without trust is not success.

---

## Institutional Responsibility

Because aerodb is backed by Tonmoy Infrastructure & Vision:

- Long-term maintenance is assumed.
- Backward compatibility rules are enforced.
- Breaking changes require explicit justification.
- Reliability claims must be defensible and measurable.

aerodb carries a name and responsibility.
It must behave accordingly.

---

## Final Statement

aerodb is built with the understanding that engineers will stake their systems,
their businesses, and their reputations on it.

We do not optimize for novelty.
We do not optimize for hype.
We optimize for trust earned through discipline.

Anything less is unacceptable.
