# Development flow

This is the default contributor and AI-assisted development flow for
`takahirox/wonderswan-rust`. The default branch is `main`.

Issue → implementation → PR → review → revision as needed → merge.
Perform post-merge verification only when required by the Issue.

## Collaboration language

Use **English** for newly authored templates, development/review/merge
documentation, AI instructions, Issue and PR titles and bodies, review comments,
and commit messages. This collaboration convention does not require translating
historical content or changing product localization.

## 1. Start with an Issue

Use the [Issue template](../.github/ISSUE_TEMPLATE/issue.md) to describe the problem,
expected outcome, relevant context, reproduction steps where applicable, and
concrete completion criteria. An assignee or owner field is not required.

Treat the source Issue as the scope of the work. Clarify genuinely blocking
ambiguity before dependent implementation; do not invent requirements.

Default to completion criteria an AI agent can execute and verify. Require human
confirmation, physical-device testing, subjective evaluation, or external approval
only when necessary for the actual Issue. Explain why each such check is needed
and what result is expected.

Separate three kinds of checks:

- **Mandatory pre-merge acceptance:** concrete criteria achievable and verifiable
  before merge, including all implementation requirements and applicable checks.
- **Optional validation:** useful additional checks that do not block completion.
- **Required post-merge verification:** genuinely necessary checks possible only
  after merge. State "None" when none are needed; do not add them routinely.

Checks possible only after merge must not be pre-merge approval conditions. For
example, merge-triggered deployment and published-site verification belong after
merge; applicable code/configuration checks and local validation belong before it.

## 2. Implement the smallest complete change

Inspect the current repository and existing guidance before editing. Preserve
useful content and justified conventions. Choose the smallest coherent design
that fully satisfies the Issue. Avoid unnecessary abstractions, speculative
extensibility, unrelated refactoring, new frameworks or subsystems, and extra
policies or configuration without a demonstrated need tied to the Issue.
Simplicity does not excuse omitted requirements or necessary validation.

Select checks appropriate to the changed files and actual tooling, following the
[test and CI performance principles](#test-and-ci-performance-principles) below.
The [README](../README.md#build-and-run-locally) documents Rust workspace checks,
reproducible demo/fixture checks, Wasm/frontend builds, and production-preview
Playwright suites. The [Pages workflow](../.github/workflows/deploy-pages.yml)
builds and deploys on pushes to `main` or manual dispatch on `main`; it does not
run the test suites, and no PR test workflow is configured. Inspect the current
tooling again when choosing validation; do not introduce tooling just to document
a process.

For template/documentation changes, check Markdown and template syntax, relative
links and anchors, whitespace, and consistency using available tools. Git provides
`git diff --check` for whitespace checks. No runtime tests, hardware/device checks,
or deployed-site verification are required for a documentation-only change unless
the actual Issue establishes a necessary reason.

### Test and CI performance principles

**Detect the required failures at the lightest appropriate layer, without
redundant execution.** Preserve required correctness, regression confidence and
meaningful integration coverage when redistributing checks.

1. **Prefer the lightest meaningful test layer.** Use Rust unit tests for CPU
   calculations, flags, branches and broad data combinations. Use native cartridge
   integration tests for emulated execution, and production-preview Playwright
   tests for Wasm, canvas, input, audio and browser storage boundaries that require
   them. Physical-console checks need an Issue-specific reason. Add tests that
   detect concrete defects, with meaningful assertions rather than copies of the
   implementation.
2. **Choose combinations deliberately.** Cover distinct paths with representative
   cases instead of automatically multiplying console models, cartridge formats,
   mappers, orientations, storage states and browser paths. Use exhaustive
   combinations where interactions require them; retain required regressions and
   justified three-model and native/browser integration coverage.
3. **Match validation to the change.** Documentation needs the syntax, link and
   whitespace checks above. Core/runtime changes need applicable Rust checks
   (`cargo fmt`, workspace tests and Clippy as documented in the README), plus
   affected cartridge/Wasm/browser integration checks. UI/Wasm changes need
   relevant Wasm/frontend builds and production-preview browser acceptance;
   demo/fixture changes need their builders' `--check` validation and affected
   consumers. CI/build changes need configuration, trigger and artifact-path
   review plus relevant local builds. Use focused checks for fast feedback, then
   complete applicable full regression validation before merge. Keep mandatory
   checks applicable to the change, identify optional checks separately, and
   record genuinely post-merge-only deployment/publication verification only
   when required by the Issue. The Pages build does not replace pre-merge tests.
4. **Reuse work and eliminate duplicate execution.** Reuse compatible builds,
   generated fixtures and downloads with cache keys reflecting toolchains,
   lockfiles and inputs; preserve isolation of browser storage and test state.
   Pages currently caches npm downloads and cancels older runs in its `pages`
   concurrency group. Future CI changes should avoid equivalent push/PR runs for
   the same revision and stop superseded work when safe while preserving necessary
   deployment ordering. Parallelize independent work only with measured resource
   use; more workers need not improve feedback time.
5. **Review test value and added cost together.** Identify the additional defect
   each new or expanded test detects, whether existing coverage already supplies
   that assurance, and its execution and maintenance cost. Watch repeated compiler
   or fresh browser setup, large ROM fixtures, long emulation loops and Cartesian
   products. Keep meaningful assertions and required coverage when reducing cost.
6. **Maintain an execution-time budget.** No normal-CI performance baseline or
   target is documented yet. Establish a repository-appropriate baseline and target
   during actual test/CI performance work, monitor growth and investigate overruns.
   Distinguish elapsed feedback time, total runner/computation time and queue time.
   Revisit validation structure and duplication as costs grow without weakening
   required coverage. This guidance adds no benchmark, hardware or merge gate.

Changes to tests, triggers, caches, worker counts or other execution settings
belong in separately scoped implementation work.

## 3. Create a PR for the Issue

Target `main` unless the source Issue specifies another base. Use the
[PR template](../.github/pull_request_template.md) to explain the problem, resulting
behavior, focused changes, related Issues, and validation actually performed.
Report results accurately, including failures, skipped checks, and unperformed
checks with reasons. Separate optional checks and pending post-merge verification
from mandatory pre-merge results.

Only claim to close an Issue when the PR fully addresses it. If intentionally
partial, explain what remains and keep the Issue open. Pending checks possible
only after merge do not make an otherwise complete implementation partial, but
must not be reported as passed before they are performed.

## 4. Review and revise

Follow the [review guidelines](review-guidelines.md). The central question is:

**Does the PR completely solve the source Issue, without unnecessary scope or complexity?**

Check both missing requirements and unjustified scope or complexity. Request
fixes for agent-fixable findings and review the updated PR again. If genuinely
necessary human input is missing, record the specific question and expected
answer, then wait through the workflow's wait path. Distinguish this state from
fixable findings instead of repeatedly requesting code fixes.

## 5. Merge and record any required follow-up

Approve and merge when the implementation is complete, mandatory pre-merge checks
are sufficient and pass, and no blocking review findings remain. Follow applicable
repository protections and the current workflow's authorization for merge actions.

Record genuinely required post-merge verification separately as pending until
performed, then report actual results, failures, and necessary follow-up. Pre-merge
validation does not prove deployment or publication succeeded. No post-merge
verification is required for this collaboration-documentation addition.

## Reference

Adapted from GitWeave's [development flow](https://github.com/takahirox/gitweave/blob/main/docs/development-flow.md),
[Issue template](https://github.com/takahirox/gitweave/blob/main/.github/ISSUE_TEMPLATE/issue.md),
and [PR template](https://github.com/takahirox/gitweave/blob/main/.github/pull_request_template.md).
