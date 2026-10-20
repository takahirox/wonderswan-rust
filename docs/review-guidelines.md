# Review and merge guidelines

Contributors and AI reviewers must read the source Issue, the PR description, the
current diff, and validation evidence. Follow the
[development flow](development-flow.md), including the
[English collaboration policy](development-flow.md#collaboration-language).

The central review question is:

**Does the PR completely solve the source Issue, without unnecessary scope or complexity?**

## Check for missing work

Compare every required behavior and acceptance criterion the PR claims to resolve
with the implementation and appropriate validation. Check correctness, relevant
documentation, and consistency with the repository's structure and conventions.
Do not approve a PR as closing an Issue when required work remains unimplemented.
An intentionally partial PR must state its limits and leave the Issue open.

## Check for unnecessary work

Require a demonstrated need tied to the Issue for added scope or complexity.
Reject unnecessary abstractions, speculative extensibility, unrelated refactoring,
frameworks or subsystems, and extra policies or configuration. Added complexity
is not automatically beneficial, including in AI-generated changes.

Prefer the smallest coherent design that fully satisfies the Issue. Preserve
justified existing conventions; do not use simplicity to excuse missing
requirements or necessary validation. Review findings themselves should not
introduce unrelated requirements or demand speculative redesigns.

## Check pre-merge acceptance and post-merge verification

Require all mandatory pre-merge acceptance criteria to be achievable and
verifiable before merge. Confirm the evidence covers the changed files with the
actual repository tooling. Check that validation reports say what was performed
and give actual results, including failures, skipped checks, and unperformed
checks with reasons. Optional checks must be identified separately.

For template/documentation changes, inspect syntax, links and anchors, whitespace,
and consistency. There are currently no configured build/test commands or CI.
Do not invent commands, demand unrelated runtime suites, or require new tooling
for this documentation-only change.

Checks possible only after merge must not block pre-merge approval. Record
genuinely required post-merge verification separately as pending until performed;
state "None" when none is needed. For example, merge-triggered deployment and
published-site checks belong after merge. Pre-merge validation does not prove
deployment or publication succeeded. This separation does not waive required
implementation work or applicable pre-merge checks.

## Review outcome and waiting

Choose the outcome that matches the evidence:

- **Agent-fixable findings:** request focused fixes, explaining the unmet
  requirement or defect and expected result. Review the updated PR again and
  verify that blocking findings are resolved.
- **Necessary human input:** record the specific question, why the answer is
  necessary, and the expected confirmation or decision. Mark the review as waiting
  for human input and use the workflow's wait path. If no automated wait path is
  available, pause dependent work with the question recorded. Resume when the
  answer arrives; do not repeatedly request code fixes for this waiting state.
- **Ready:** approve when the implementation fully addresses the claimed Issue
  scope, no unjustified complexity or blocking findings remain, and all mandatory
  pre-merge checks are sufficient and pass. Merge after approval, subject to
  repository protections and the current workflow's authorization.

Pending checks possible only after merge do not prevent approval. After merge,
perform any required post-merge verification and record actual results, including
failures and follow-up. Keep unperformed checks pending. This templates and
documentation addition requires no post-merge verification.

## Reference

Adapted from GitWeave's [review guidelines](https://github.com/takahirox/gitweave/blob/main/docs/review-guidelines.md).
