---
name: speckit
description: Spec-driven development toolkit for this project. Use when the user invokes any `/speckit-*` slash command (speckit-constitution, speckit-specify, speckit-plan, speckit-tasks, speckit-implement, speckit-analyze, speckit-clarify, speckit-checklist, speckit-converge). Provides the full spec-kit workflow for turning ideas into code via constitution → spec → plan → tasks → implement → converge.
---

# Spec-Kit Workflow

This skill orchestrates the spec-kit development lifecycle. The workflow is designed to be invoked via the following slash commands:

## Workflow Commands

| Command | Purpose | Input | Output |
|---------|---------|-------|--------|
| `/speckit-constitution` | Establish governing principles | Codebase analysis | `.specify/memory/constitution.md` |
| `/speckit-specify` | Create feature specification | Requirements description | `FEATURE_DIR/spec.md` |
| `/speckit-clarify` | Resolve ambiguities in spec | Current spec.md | Updated `spec.md` |
| `/speckit-plan` | Design implementation architecture | spec.md | `FEATURE_DIR/plan.md` |
| `/speckit-tasks` | Generate actionable task list | spec.md + plan.md | `FEATURE_DIR/tasks.md` |
| `/speckit-analyze` | Cross-artifact consistency check | spec/plan/tasks | Analysis report (read-only) |
| `/speckit-checklist` | Requirements quality checklist | spec/plan/tasks | `FEATURE_DIR/checklists/*.md` |
| `/speckit-implement` | Execute implementation | tasks.md | Production code + tests |
| `/speckit-converge` | Gap analysis post-implementation | Codebase vs spec/plan/tasks | Updated `tasks.md` (append-only) |

## Recommended Flow

```
constitution → specify → clarify → plan → tasks → analyze → implement → converge
                                                                    ↘ checklist ↗
```

## Key Directories

- `.specify/` — Spec-kit templates, scripts, memory
- `.codebuddy/commands/` — Custom slash command definitions
- `.codebuddy/skills/speckit/` — This master skill definition

## Important Rules

1. **Constitution is non-negotiable** — every step must validate against it.
2. **APPEND-ONLY** for converge (`tasks.md`) and checklist (if file exists).
3. **READ-ONLY** for analyze — never modify artifacts during analysis.
4. **One question at a time** for clarify — max 5 questions per session.
5. **Phase order matters** for implement — always process tasks sequentially within phases.
