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

Select checks appropriate to the changed files and actual tooling. At the time
this guidance was added, the repository contains collaboration documentation and
templates only: no application files, build/test commands, or CI are configured.
Do not invent commands or introduce application scaffolding, dependencies, CI, or
build/test tooling just to document a process. Inspect again for future changes.

For template/documentation changes, check Markdown and template syntax, relative
links and anchors, whitespace, and consistency using available tools. Git provides
`git diff --check` for whitespace checks. No runtime tests, hardware/device checks,
or deployed-site verification are required for a documentation-only change unless
the actual Issue establishes a necessary reason.

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
