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
An intentionally partial PR must state its remaining scope and leave the parent
Issue open with a non-closing reference. Pending post-merge-only verification does
not make an otherwise complete implementation partial.

For interface changes, review against the [UI/UX principles](../README.md#uiux).

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

Apply the [test and CI performance principles](development-flow.md#test-and-ci-performance-principles):
review the additional defects covered, duplicated assurance/execution, and added
execution and maintenance cost of new or expanded checks. Ask:

**Does this change provide sufficient confidence with the lightest appropriate
validation, without redundant execution?**

For template/documentation changes, inspect syntax, links and anchors, whitespace,
and consistency. For code/build changes, use the applicable Rust, fixture,
Wasm/frontend and production-preview browser checks documented in the
[README](../README.md#build-and-run-locally). The Pages workflow builds and deploys
on `main` but does not run the test suites. Do not demand unrelated runtime suites
or require new tooling for documentation-only changes.

Checks possible only after merge must not block pre-merge approval. Record
genuinely required post-merge verification separately as pending until performed;
state "None" when none is needed. For example, merge-triggered deployment and
published-site checks belong after merge. Pre-merge validation does not prove
deployment or publication succeeded. This separation does not waive required
implementation work, mandatory pre-merge tests, required protections, or genuinely
necessary pre-merge human acceptance.

Preserve `Closes #N` when the implementation is complete and applicable mandatory
pre-merge acceptance is satisfied. Do not request removing it, keeping the source
Issue open, or withholding approval solely for pending post-merge-only checks.
ProjectWeave publication adds this reference; asking review/fix to remove it
creates a Review → Fix → Publish → Review loop. Follow the
[merge and follow-up lifecycle](development-flow.md#5-merge-and-record-any-required-follow-up).

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

Pending checks possible only after merge do not prevent approval. After successful
merge, close the original Issue, then perform required post-merge verification.
Record actual evidence in the merged PR, closed Issue, or linked validation report;
keep pending, unperformed, and environment-blocked checks explicit. Closure does
not claim they passed, and an unavailable environment alone is not evidence of a
product defect. For actual defects, check for an equivalent Issue before creating
a new linked Issue with reproduction steps, affected revision, expected/actual
behavior, and original Issue/PR links. Do not reopen the original Issue solely for
follow-up.
The linked lifecycle supersedes older keep-open instructions without rewriting
historical results. This documentation/template change requires no post-merge
verification.

## Reference

Adapted from GitWeave's [review guidelines](https://github.com/takahirox/gitweave/blob/main/docs/review-guidelines.md).
