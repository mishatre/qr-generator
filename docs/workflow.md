# Workflow v1

## What this workflow optimizes

Deliver a usable change while keeping your reading effort and account usage manageable. Keep future ideas without implementing them prematurely. A modular design can reduce the cost of later changes; no plan can guarantee that future work will never require refactoring.

The unit of work is one demonstrable outcome. For example: “GISP service consumes replacement open-data input.” “Build a reusable ingestion framework” is a separate project, not a hidden prerequisite.

## Planning and execution are different activities

A planning conversation can be exhaustive. Its output is a durable map of outcomes, dependencies, decisions and future ideas. Keep only the next one or two tasks implementation-ready; later tasks may remain short outlines. Overdetailing distant tasks creates stale plans and spends quota before requirements are known.

A ready task includes a small implementation path, concrete acceptance examples, relevant source locations, verification commands, release target, and stop conditions. It must be understandable from its first screen.

Use Plan mode to investigate and settle scope. If that client cannot write documents in Plan mode, complete planning first, then send a normal-mode request to save the agreed documents only. Do not implement application code merely to save a plan.

Use normal execution mode for a ready task. Goal mode is optional for a bounded outcome that needs persistence; it is not a quota discount. Do not make “finish the roadmap” the goal. Preserve the task's explicit review and blocker boundaries.

## Create a task versus plan a project

Use “Create a task for…” to capture a new feature or fix in an existing project.
The coordinator follows `docs/workflow-prompts/05-create-task.md`, checks existing
tasks for duplicates, assigns the next unused ID, saves a draft and updates the
index. Keep acceptance examples, relevant code locations and unknowns in the task.
Do not elaborate the whole roadmap or implement source changes for this request.

Use “Refine task T001” to settle implementation details later. “Create and refine
a task for…” can do both documentation steps in one request. “Plan this project”
is for the broader purpose, architecture and roadmap. Prompt numbers are stable
file identifiers, not the required execution order.

## Files in each project

- `AGENTS.md`: durable readability, scope and workflow rules; small enough to read routinely.
- `docs/plan/project.md`: purpose, current need, architectural boundaries and future direction.
- `docs/plan/index.md`: task IDs, status, dependencies and the next recommended task.
- `docs/plan/tasks/T001-short-name.md`: task card, implementation context and completion evidence in one file.
- `docs/plan/environment.md`: working directories, relevant commands and actual validation/deployment capabilities.
- Existing README or feature docs: current behavior and how to use it, updated alongside delivery.

Use the task file as the single source of truth for scope and evidence. Do not duplicate it into OBJECTIVE, STATUS, HANDOFF and agent-log files. Link important decisions from the project brief; add a dedicated decision document only when it needs a durable explanation.

Statuses: `draft → ready → in_progress → verified → delivered`. `blocked` records an unmet prerequisite; `deferred` records intentionally later work. Source checked, compiled, runtime checked and deployed are separate evidence fields. A task cannot be verified if required acceptance checks are missing. If checks are unavailable, report that and retain an honest pending/blocked status.

## The decision card shown in chat

Aim for 120 words or fewer, unless the user requests detail:

- Outcome and why it matters now.
- Included work and deliberately deferred ideas.
- How success will be demonstrated.
- Next action, or one unresolved decision with a recommendation.

Detailed material lives below the card in the task file. Ask ordinary chat questions, preferably one at a time, with no timer. Absence of an answer is not approval. Required unanswered questions remain pending across pauses and resumption. Continue independent safe preparation while a required decision is pending.

## Model routing: starting settings

| Role | Model / effort | Work |
|---|---|---|
| Coordinator | Sol Medium | Scope, task selection, delegation, integration and final delivery report |
| Normal worker | Terra Medium | One bounded implementation; targeted tests; relevant documentation |
| Mechanical worker | Luna Medium | Exact repetitive edits, fixtures or documentation with clear examples |
| Reviewer | Sol Medium | Independent acceptance, correctness and readability review |
| Specialist | Sol High | Unfamiliar integration behavior, substantial 1C reasoning, recurring failure |
| Architecture escalation | Astra Medium | One unresolved architectural decision with alternatives and evidence |
| QA specialist, optional | Terra Medium | Independent UI/runtime scenario or large test-log investigation |

These are hypotheses, not benchmark winners. Use the client's supported model IDs and effort values; record the actual selected model if observable. If the routing request cannot be honored, report it. Do not pretend text instructions changed the main conversation's model.

## Three execution recipes

**Tiny:** one local correction with clear behavior and little interaction. Terra Medium directly implements and verifies it. No compulsory plan artifact or separate reviewer. Use a task file if continuation or release tracking matters.

**Normal — default:** Sol coordinates; one Terra worker implements and tests; a fresh Sol reviewer inspects the resulting diff and acceptance evidence; the worker fixes concrete findings; Sol reports the result. Reviewer reads actual source, tests and relevant surrounding contracts, not just a worker summary. Coordinator resolves findings and confirms required evidence without repeating every successful test.

**Complex:** first refine into smaller ready tasks. Add a specialist only for a distinct uncertainty or independent QA scenario. Use Sol High for unfamiliar 1C/integration work initially, then tune from observed results. Astra examines a bounded design question, not every command and test run.

Manager, gatekeeper and scheduling duties belong to the coordinator. Tests are commands first, not a compulsory extra agent. There is no perpetual auditor or security-cleanup phase. Check relevant security properties during implementation and review.

## Delegation contract

Only the top-level coordinator runs this pipeline. Delegated agents do their assigned work without starting another pipeline.

Send each child: task ID/path, exact objective, relevant files/symbols, acceptance examples, allowed writes, known constraints, verification commands and expected short evidence summary. Start with focused context; request more when dependencies require it. Do not broadcast the entire conversation or roadmap by default.

At most two active subagents; one code writer at a time by default. Workers do not spawn more workers. Parallelize independent read-only investigation or QA preparation when useful. QA against a changing checkout waits for the writer to finish or uses an isolated snapshot. Do not let concurrent tasks mutate a shared test database or 1C infobase.

After implementation, review the stable diff. Correct findings in the same worker context when available. After two failed attempts at the same acceptance failure, stop blind retries: coordinator narrows the reproduction and revises the approach, or consults one specialist. If no evidence-backed next step exists, save a blocker and ask the one necessary question. Never call unfinished work complete because the retry budget was reached.

If subagents are unavailable, a single agent may implement and run an explicitly labeled self-review, preserving the same checks. Report that review was not independent; if independent review is required by the task, leave that criterion pending for a separate review conversation. If a requested model is unavailable, use an already-approved available route or ask one untimed question; do not silently substitute a costlier model.

## Scope and future-proofing

Classify new ideas as needed now, later, or undecided. Only needed-now requirements enter the active task. Add future ideas to the index without implementing them. If new information makes the agreed outcome impossible, revise scope explicitly.

Preserve a simple boundary for likely growth: e.g. an async iterable of records between download/parsing and the service. Do not add adapters, hooks, compatibility layers or configuration merely because a future consumer might need them. Record the extension direction and reconsider when a real second use case arrives.

Use one representative consumer example before implementation to expose a bad API cheaply. Review the main execution path before broadening the library. A task should leave the system usable or provide a clearly necessary, independently verifiable prerequisite; avoid tiny technical subtasks that produce only orphan scaffolding.

## Readability is acceptance

Use descriptive domain names, braces in languages that use them, and blank lines between logical steps. Prefer straightforward branches and loops over dense chains or nested expressions. Extract named operations when they express a responsibility, not just to satisfy a line count. Keep related behavior close. Explain surprising decisions, invariants and data transformations; do not narrate obvious syntax.

Keep the public path easy to trace: entry point → main operations → output. Review should flag needless indirection and compressed code. Finish with a three-to-five-sentence walkthrough and the location of the likely next change. A formatter/linter should enforce mechanical rules when supported, without wholesale unrelated reformatting.

## Validation and delivery

Record actual commands, results and limitations. Start with a representative acceptance fixture and consumer-level smoke check. Run required project checks and tests covering changed behavior; broaden for new evidence or failures. Do not rerun passing checks mechanically in every role. Recheck after relevant code changes.

For admin UI work include the relevant real interaction, error/empty state and API integration. Use browser QA when available; compilation alone is not UI verification.

For 1C, use the repository's established export/build tools and format. Record artifact kind, platform/configuration versions, source-of-truth location, export command, compile command, output path and target test infobase. Separate: source/static checks; compilation; load and execution; business/integration scenario. No build or runtime tooling means explicitly unverified stages, not invented results. Keep generated export churn separate from intended changes. Preserve the original artifact and target a distinct output or established reversible build process. Actual platform commands must come from the environment, not guessed templates.

A change touching stock, transactions or inter-system messages gets checks for the relevant duplicate/retry/partial-failure behavior even if the project is generally tolerant of small bugs.

Define delivery in the task: local artifact, consumer integration, package publish, deployment, or manual import. Choose the shortest established route that reaches the real consumer. Public npm publishing is not automatically required for an internal urgent fix. Prepare and verify the concrete artifact; execute publishing/deployment when authorized by the task and allowed by the environment. Otherwise mark it awaiting that action, with exact instructions. Routine in-scope work does not require repeated permission.

## Pauses and measurement

One execution conversation per ready task by default; keep related corrections there. Use a new conversation for a different task. The fresh reviewer subagent already satisfies independent review; a manually opened review conversation with a concise handoff is an alternative, not an additional compulsory review. Do not reset context at every step automatically.

Before stopping, update the task with completed work, exact check results, remaining issue and next command/action. On resume verify the working-tree state before trusting the previous evidence. Do not restart planning from scratch.

For the first three normal tasks, record model/effort, elapsed time, quota before/after if visible, repair count and approximate human review minutes. Other concurrent account work confounds quota deltas; say so. Do not ask the model to invent exact token accounting or enforce a hard percentage stop without telemetry. Optimize for delivered tasks and review effort, not raw token count. Change one routing choice at a time. No benchmark reruns of whole projects unless the user wants them.
