# Requirements Management Principles

Effective requirements management is built on several foundational principles. Understanding these helps you structure your requirements for maximum value.

## What is a Requirement?

A **requirement** is a documented statement of a need, constraint, or capability that a system must satisfy. Requirements answer the question: "What must the system do or be?"

Good requirements are:

- **Clear** - Unambiguous and understandable by all stakeholders
- **Testable** - You can verify whether the system satisfies the requirement
- **Necessary** - The requirement addresses a genuine need
- **Feasible** - The requirement can realistically be implemented
- **Traceable** - The requirement can be tracked through the development lifecycle

## Requirement Levels

Requirements typically exist at multiple levels of abstraction:

### User Requirements

High-level statements of user needs and goals. Written in user terminology, these describe *what* users need to accomplish, not *how*.

Example: "Users shall be able to export data in multiple formats."

### System Requirements

Technical specifications derived from user requirements. These describe *how* the system will satisfy user needs.

Example: "The system shall provide export functionality supporting CSV, JSON, and XML formats."

### Software/Hardware Requirements

Detailed requirements for specific subsystems or components.

Example: "The export module shall use the serde library for JSON serialization."

## Requirement Attributes

Each requirement should have metadata:

- **Unique Identifier** - A stable reference (e.g., USR-001)
- **Status** - Draft, approved, implemented, verified, etc.
- **Priority** - Critical, high, medium, low
- **Owner** - Person or team responsible
- **Rationale** - Why this requirement exists
- **Source** - Where the requirement came from

*Note: Requiem currently stores identifiers and timestamps. Status, priority, and other attributes can be added as tags or in the requirement body.*

## Traceability

**Traceability** is the ability to track relationships between requirements and other artifacts (design documents, code, tests). There are two types:

- **Forward traceability** - From requirements to design, code, and tests
- **Backward traceability** - From code and tests back to requirements

Traceability enables:

- Impact analysis (what breaks if we change this requirement?)
- Coverage analysis (are all requirements implemented and tested?)
- Compliance verification (can we prove we met all requirements?)

## Requirement Dependencies

Requirements rarely exist in isolation. They form a **directed graph** where:

- **Parent (upstream) requirements** - Higher-level needs that must be satisfied
- **Child (downstream) requirements** - Detailed specifications that satisfy parents

Example:
```
USR-001: "Users shall authenticate securely"
  └─ SYS-001: "System shall use OAuth 2.0"
       └─ SWR-001: "Use the oauth2-rs library"
```

## The V-Model

The **V-Model** visualizes how requirements flow through development:

```
User Requirements  ←→  Acceptance Tests
       ↓                      ↑
System Requirements ←→ Integration Tests
       ↓                      ↑
Software Requirements ←→ Unit Tests
       ↓                      ↑
    Implementation
```

Each level of requirements corresponds to a level of testing that verifies those requirements.

## Requirements vs. Design

A common challenge is distinguishing requirements from design:

- **Requirement** - WHAT the system must do
- **Design** - HOW the system will do it

Example:
- ❌ Requirement: "The system shall use a PostgreSQL database" (This is design)
- ✅ Requirement: "The system shall persist data reliably across restarts" (This is a requirement)
- ✅ Design: "We'll use PostgreSQL to satisfy the persistence requirement" (This is design)

However, in some contexts (particularly lower levels), the line blurs. "System shall use OAuth 2.0" might be a legitimate requirement if it's mandated by stakeholders or regulations.

## Change Management

Requirements change. Good requirements management accepts this and provides mechanisms to:

1. **Track changes** - Know what changed, when, and why
2. **Analyze impact** - Understand what's affected by a change
3. **Trigger reviews** - Ensure dependent requirements and tests are updated
4. **Maintain history** - Preserve the evolution of requirements

## Validation vs. Verification

Two distinct but related concepts:

- **Validation** - Are we building the right thing? (Do requirements match user needs?)
- **Verification** - Are we building the thing right? (Does implementation match requirements?)

Validation often involves stakeholder reviews. Verification involves testing and inspection.

## Why These Principles Matter

Following these principles provides:

- **Clarity** - Everyone understands what's being built
- **Accountability** - Clear ownership and traceability
- **Quality** - Testable requirements lead to better testing
- **Agility** - Understanding impact enables confident change
- **Compliance** - Proof that requirements are met

## Requiem's Approach

Requiem embodies these principles through:

- Plain-text markdown for clarity and accessibility
- YAML frontmatter for metadata and relationships
- UUID-based traceability with human-readable aliases
- Content fingerprinting for change detection
- Multiple parent support for complex dependencies

Continue to [Traceability](./traceability.md) to see how Requiem implements these concepts in practice.
