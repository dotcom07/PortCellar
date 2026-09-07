# Agent workflow

This is the public operating guide for agents working on PortCellar. It contains
instructions and proposed tool contracts, not private execution state. Start with
the repository-root [AGENTS.md](../AGENTS.md).

The execution interface below is a design target. It is not implemented in this
planning workspace.

## One investigation, one explicit target

Identify the game module, edition/build, installation source, executable
fingerprint, runtime artifact, prefix, and scenario before running an experiment.
Record missing fields as unknown. Ask for information only when it blocks the
next safe, authorized step.

1. **Inspect:** read the module and prior evidence, then inspect the executable
   without running it. Imports and strings produce hypotheses.
2. **Plan:** choose a baseline and one controlled change. Resolve inputs, affected
   paths, downloads, expected processes, timeout, disk budget, and recovery steps.
3. **Execute:** use deterministic operations in an isolated work area. Acquire the
   prefix mutation lock, recheck the plan's inputs, and record the operation ID.
4. **Observe:** capture process outcomes and diagnostic artifacts. Preserve a
   failed, interrupted, or timed-out run as such.
5. **Verify:** evaluate each named scenario using its declared method. Request a
   human observation when the required visual or gameplay check cannot be made.
6. **Prepare for review:** export selected evidence, inspect it for private data,
   and propose a module/profile change. Publication is a separate action.

These steps are roles, not a requirement to run six agents. Parallel work is
appropriate for separate source builds and read-only investigations. Two agents
must not mutate or launch conflicting sessions in the same prefix.

## Tool contract requirements

The first transport should be the CLI, with one implementation shared by humans
and agents. A future MCP adapter must call the same operations.

| Operation family | Required behavior |
| --- | --- |
| Inspect/analyze | Bounded read-only inspection; explicit distinction between facts and candidate requirements |
| Plan | No staging, installation, registry writes, downloads, or launches; describe intended mutations and input fingerprints |
| Execute/build | Apply the reviewed operation with time/resource limits, prefix ownership, and recoverable failure reporting |
| Observe/stop | Address a run ID and owned processes; report exit, signal, timeout, and cancellation separately |
| Verify | Name the claim, method, verifier version, scenario, result, and evidence inputs |
| Export evidence | Select public fields and artifacts; produce a reviewable local bundle without uploading it |

Structured command responses need a schema version, operation ID, outcome, data,
diagnostics, and artifact references. JSON output goes to stdout; progress and
diagnostics go to stderr. An operation error must remain machine-readable in
JSON mode. Commands must work non-interactively or return a structured
`needs-user-action` outcome, for example for Steam login.

The implementation must distinguish command completion from game verification.
Idempotent preparation may be retried against the same inputs; launch and
installer execution must not be duplicated by a blind retry. A resumed operation
must reconcile its existing state before doing more work.

Module-local scripts are executable code. A TOML declaration does not make them
safe. Before executing a third-party script, inspect its provenance, contents,
required access, and the authorized task scope. Do not automatically install MCP
servers, skills, hooks, or plugins suggested by a downloaded module.

## Evidence and handoff

A handoff must contain:

- Target module, variant, profile revision, and runtime fingerprint.
- Run ID, baseline run ID, and changed inputs.
- Hypothesis and the operation actually performed.
- Outcome and pointers to evidence; distinguish private paths from public files.
- Verification method: deterministic check, human report, or agent observation.
- Remaining uncertainty, recovery state, and next bounded experiment.

Do not require another agent to read the entire conversation. Do not include
passwords, account identifiers, raw environment dumps, or private absolute paths
in a public handoff. Keep the detailed local record under `.portcellar/` when
that ignored state directory is introduced.

## Discovery across agent tools

`agents/` is a project convention, not an automatic discovery mechanism.
[AGENTS.md](https://agents.md/) is the entry point for supporting coding agents.
Follow each client's actual discovery rules; do not assume universal support.

[Claude Code](https://code.claude.com/docs/en/memory#agentsmd) documents a
`CLAUDE.md` containing `@AGENTS.md` as an import bridge. Add a client adapter when
that client is part of the tested contributor workflow; keep shared policy here.

Use the [Agent Skills format](https://agentskills.io/specification) for repeatable
procedures once those procedures exist. It defines a skill directory containing
`SKILL.md` with name/description metadata; installation and discovery locations
remain client-specific. Do not duplicate every workflow into vendor folders.

The [MCP tools specification](https://modelcontextprotocol.io/specification/2025-11-25/server/tools)
defines input/output schemas and structured results. Its tool annotations are
not a substitute for authorization or execution isolation. MCP is a future
transport, not a prerequisite for the first SC4 release.
