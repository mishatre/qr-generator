# Review and create follow-up tasks

Review [TASK / FEATURE / PATH / BASE COMMIT]. Use Sol Medium as coordinator and one independent Sol Medium reviewer for a substantial change. Read project context and actual implementation. If the review target is ambiguous, ask one untimed question. Do not modify application source merely because a review found something.

Inspect correctness, the real consumer path, relevant tests, readability and scope. Evaluate security properties relevant to this change. Cite concrete files/symbols and explain observable consequences. Separate actual defects, missing evidence and optional improvements. Report no findings if no actionable issue is supported. Do not demand speculative abstractions or new features.

For a feature involving 1C or integrations, distinguish source, build and runtime evidence. For UI, identify whether real interaction was checked. Use a specialist only for a specific unresolved risk, not a universal checklist.

Save a concise review record in the task, or docs/plan/reviews for a broader audit. Create draft follow-up tasks for actionable findings, link evidence, and order them by impact. Mark optional improvements deferred. Reuse existing tasks to avoid duplicates. Return at most three leading findings and the next recommended action; keep the rest in files. Review does not authorize fixes or deployment unless explicitly requested.
