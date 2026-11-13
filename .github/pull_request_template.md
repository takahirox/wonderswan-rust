<!-- Write the title and body in English. -->

## Problem and summary

Describe the source Issue's problem and the focused changes that address it.

## Outcome

Describe the resulting behavior, including before/after behavior when useful.
Explain how the change covers the Issue's required scope. If intentionally partial,
state what remains and do not claim to close the Issue.

## Validation

List checks actually performed and their results against mandatory pre-merge
acceptance. Report failures, skipped checks, and unperformed checks with reasons.
Choose checks appropriate to the changed files and existing repository tooling.
Report optional validation separately; do not imply unperformed checks passed.

## Required post-merge verification

State "None" unless required by the Issue. Otherwise list checks possible only
after merge, expected results, and pending status until performed. These checks
are separate from pre-merge approval; do not infer success from pre-merge results.
For a fully implemented Issue with mandatory pre-merge acceptance satisfied, close
it after successful merge, then perform these checks. Record evidence and check
status, and track actual defects in an existing equivalent or new linked Issue,
following the [merge and follow-up lifecycle](https://github.com/takahirox/wonderswan-rust/blob/main/docs/development-flow.md#5-merge-and-record-any-required-follow-up).

## Related Issues

Link the source Issue and any related Issues. Use and preserve `Closes #<number>`
when this PR fully implements that Issue and satisfies applicable mandatory
pre-merge acceptance. Pending post-merge-only verification does not make the work
partial or require removing this reference. For intentionally partial work, state
the remaining scope above and use a non-closing reference to the parent Issue.

See the [development flow](https://github.com/takahirox/wonderswan-rust/blob/main/docs/development-flow.md) and
[review guidelines](https://github.com/takahirox/wonderswan-rust/blob/main/docs/review-guidelines.md).
