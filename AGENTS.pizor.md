# Pi Multi-Node Orchestration Policy

This project may run multiple Pi instances inside one Zellij session
This file is the shared policy for every Pi instance in this repo

## Roles

There are three logical node roles:

- observer node: monitors lifecycle, drift, progress, cleanup, and keeps polling out of the user/main thread
- master node: decomposes tasks, spawns workers, kills workers, manages global context
- worker node: executes exactly one bounded assignment and reports back

DO NOT use metaphor-specific names in commands, session names, scripts, skills, task IDs, or reports

Use neutral names only:

- node-observer
- node-master
- node-worker
- pizor
- worker-spawn
- worker-dispatch
- worker-kill

## Dynamic Worker Policy

All workers are ephemeral
The master MUST NOT maintain a fixed pool of idle workers

DO NOT create standing workers such as:

- investigation worker
- implementation worker
- test worker
- review worker

Spawn a worker only when there is a concrete bounded assignment

Prefer safe parallel batches over unnecessary sequential execution. Implementation, read-only investigation, read-only review, and verification assignments may run at the same time when their scopes do not conflict.

A concrete assignment must include:

- task_id
- assignment_id
- objective
- scope
- stop condition
- required report format

A worker SHOULD receive exactly one assignment
After a worker reports done, blocked, failed, cancelled, or needs_review, the master must capture the report and close the worker pane unless there is a clear reason to keep it temporarily

## Context Policy

DO NOT allow long linear context growth
DO NOT wait until context usage reaches 50%

Prefer `/tree` when:

- switching strategy
- abandoning a noisy branch
- comparing alternatives
- ingesting worker reports
- synthesizing multiple assignment results
- carrying forward only selected context
- recovering from failed attempts

Prefer `/compact` only when continuing the same chosen branch is clearly better than returning to a checkpoint.

## Master Context Policy

The master owns global context
The master MUST NOT paste all raw worker logs into one long context

The master SHOULD use `/tree` to keep separate branches for:

- planning
- dispatch batch
- individual worker report
- synthesis
- final decision

The master should summarize each worker report branch with custom focus instructions.

The master carries forward only:

- assignment_id
- worker_pane_id
- files inspected
- files modified
- commands run
- confirmed findings
- blockers
- recommendation
- context_to_carry_forward
- context_to_discard

The master discards:

- raw logs unless essential
- repeated reasoning
- abandoned attempts
- speculative discussion
- stale plans

## Worker Context Policy

A worker owns only one assignment

A worker must:

- load `/skill:node-worker`
- acknowledge task_id and assignment_id
- create or identify a local `/tree` checkpoint before substantial work
- avoid expanding scope
- avoid irreversible changes unless explicitly allowed
- report structured results
- stop and wait for cleanup after reporting

A worker MUST NOT assume it shares context with the master

Workers must explicitly report reusable facts.

## Observer Policy

The observer monitors dynamic worker lifecycle
The observer does not assume fixed workers
The observer SHOULD run `.pi/scripts/observer-loop` or an equivalent loop so the user/main thread does not repeatedly sleep and dump panes

The observer tracks:

- task_id
- assignment_id
- worker_pane_id
- status
- purpose
- blocker
- cleanup_required
- report_captured

The observer SHOULD warn the master when:

- a worker is idle
- a worker has completed but has not been killed
- a worker expands scope
- workers duplicate work
- reports are missing required fields
- synthesis is accumulating raw logs
- the master should switch to `/tree`
- only one worker is active despite obvious non-conflicting review, investigation, or verification work
- the master runs tests, lint, review, or final verification directly instead of delegating to a worker

## Structured Message Policy

All inter-node messages MUST use bracketed message blocks

Allowed message types:

- MASTER_DISPATCH
- WORKER_REPORT
- OBSERVER_STATUS
- MASTER_CONTROL
- WORKER_CONTROL

Every message MUST include:

- from_pane
- to_pane
- role
- message_type
- task_id when applicable
- assignment_id when applicable
- status
- payload or structured sections

DO NOT send vague progress updates

## Assignment Status Values

Use only these status values unless the user explicitly changes the protocol:

- assigned
- acknowledged
- running
- done
- blocked
- failed
- cancelled
- needs_review

## Report Requirements

Every worker report must include:

- summary
- files inspected
- files modified
- commands run
- findings
- blockers
- recommendation
- context_to_carry_forward
- context_to_discard

## Safety and Scope

DO NOT run destructive commands unless the assignment explicitly allows them
DO NOT commit changes unless the assignment explicitly asks for commits
DO NOT modify unrelated files
DO NOT broaden the task without asking the master
DO NOT coordinate with other workers unless instructed by the master

## Verification and Review Policy

For substantial changes, review and verification are worker assignments, not master chores.

- The master SHOULD spawn a read-only review worker before final synthesis.
- The master SHOULD spawn a verification worker for project checks such as tests, lint, and formatting.
- The master SHOULD NOT run final verification commands directly unless no worker can be spawned.
- The observer SHOULD warn if final synthesis begins without review or verification reports.

