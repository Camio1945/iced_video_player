---
description: Create or update the project constitution with governing principles, providing guardrails for all subsequent development.
argument-hint: [optional update description]
---

You are tasked with creating or updating a project constitution.

## Context

The constitution is the foundational governance document for this project. It defines core principles that are **non-negotiable** and must guide all design, implementation, and review decisions.

## Steps

1. **First**, check if `.specify/memory/constitution.md` exists:
   - If it exists, load it — you are updating it (preserve the version number and bump appropriately).
   - If it does not exist, load `.specify/templates/constitution-template.md` — you are creating from scratch.

2. **Gather project context**: Read `README.md`, `Cargo.toml`, and scan the codebase to understand the project's identity, tech stack, and architectural patterns.

3. **Draft the constitution** following the template structure:
   - Each principle must have: a concise name, a **MUST** / **SHOULD** / **MUST NOT** normative statement, and a rationale.
   - Cover: architecture, code organization, testing, performance, security, and the project's unique constraints.

4. **Validate** against the codebase — every principle must accurately reflect the project's actual or intended practices (not aspirational ideals that don't match reality).

5. **Write** the final constitution to `.specify/memory/constitution.md`.

6. **Report**: version number, number of principles, any recommended follow-up commands (e.g., `/speckit-specify`).
