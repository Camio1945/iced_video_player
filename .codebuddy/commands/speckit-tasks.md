---
description: Generate actionable, dependency-ordered implementation tasks (tasks.md) from the plan, with parallel markers and precise file paths.
argument-hint: [additional task requirements]
---

You are tasked with generating implementation tasks from the plan.

## Prerequisites

1. Run `.specify/scripts/powershell/check-prerequisites.ps1 -Json` from repo root.
2. Load `spec.md`, `plan.md`, and `constitution.md` from FEATURE_DIR.
3. Load `.specify/templates/tasks-template.md` for structure.

## Steps

### 1. Parse the Plan
- Extract all phases, components, and files from `plan.md`.
- Map each plan decision to concrete work items.

### 2. Generate Tasks
For each phase, create tasks that are:
- **Atomic**: One clear action per task (single file, single concern).
- **Verifiable**: Has a clear "done" condition.
- **Traceable**: References the source requirement (FR-###, US-##, SC-###).
- **Ordered**: Dependencies flow from top to bottom.

**Task format:**
```
- [ ] T### [P] <action> in <filepath> per <source-ref>
```
- `[P]` marker for tasks that can run in parallel.
- Zero-padded IDs: T001, T002, ...

### 3. Dependency Ordering
- Foundation tasks first (setup, types, interfaces).
- Dependent tasks after their prerequisites.
- Group by phase with `## Phase N: <name>` headers.
- Maximize parallelization with `[P]` markers.

### 4. Constitution Check
Refer back to constitution — ensure no task would violate principles.

### 5. Write tasks.md
Write to `FEATURE_DIR/tasks.md`.

### 6. Report
Output: tasks path, task count, phase count, parallel task count, and suggested next command (`/speckit-implement` or `/speckit-analyze`).
