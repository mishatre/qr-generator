# Create a task

Create a task for [FEATURE / PROBLEM / REQUEST]. Use Sol Medium. This request
authorizes task documentation, not application implementation or deployment.
Follow the project AGENTS.md; all paths below are relative to this project root.

Read docs/plan/index.md, the relevant project context and
docs/plan/templates/task.md. Inspect only the source needed to understand the
request and name useful entry points. Check the index and task filenames for
related work and used IDs. If an existing task already covers the same outcome,
update or link it instead of creating a duplicate; preserve its existing scope
and evidence, and report what you did.

Identify the immediate useful outcome. Separate required-now behavior from
future ideas. Create one small, demonstrable task; if the request spans several
independent outcomes, create a small set of linked draft tasks and recommend
which comes first. Do not create a full project roadmap for one feature.

Assign the next unused T-number, checking both index and task files, without
renumbering existing tasks. Save docs/plan/tasks/TNNN-short-name.md using the
local template and update docs/plan/index.md. Create missing document directories
when needed. Do not overwrite existing tasks. Record a short decision card,
initial acceptance examples, in/out scope, relevant files, dependencies, delivery
target and known unknowns. Keep unverified commands explicitly unverified.

Default to draft: creation captures work; refinement makes it implementation-ready.
If the user explicitly asks to create AND refine it, follow
docs/workflow-prompts/02-refine-task.md after saving. Neither operation authorizes
implementation. Do not run worker/reviewer teams just to capture a task.

Save useful context even when something is unclear. Ask one ordinary untimed
question only when a material decision is needed; keep it pending across pauses.
Never treat silence as agreement. Put detailed context in the file and return
only the task ID, short outcome, file path, and next recommended action.

User request:
[Describe what you need. Include the immediate outcome if known.]
