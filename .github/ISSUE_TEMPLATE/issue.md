---
name: Issue
about: Report a problem or propose a change
title: ""
labels: ""
---

<!-- Write the title and body in English. See the development guidance below. -->

## Problem

Describe the problem and why it matters.

## Expected outcome

Describe what should be true when the Issue is resolved.

## Context and reproduction

Add relevant context, examples, logs, related Issues, and reproduction steps where
applicable. Omit reproduction steps when they do not apply.

## Pre-merge acceptance criteria

- [ ] Replace this with a concrete, mandatory criterion achievable and verifiable
  before merge. Add criteria as needed to cover the expected outcome completely.

Default to criteria an AI agent can execute and verify. Require human confirmation,
physical-device testing, subjective evaluation, or external approval only when
necessary for this Issue; explain why and what result is expected. No assignee or
owner field is required. See the [development guidance](https://github.com/takahirox/wonderswan-rust/blob/main/docs/development-flow.md#1-start-with-an-issue).

Checks possible only after merge must not be prerequisites for pre-merge approval.
Keep all implementation requirements and applicable pre-merge checks.

## Optional validation

List useful additional checks separately from mandatory acceptance, or state
"None." Optional checks do not block completion.

## Required post-merge verification

State "None" unless the Issue requires checks possible only after merge. If
required, describe the checks and expected results; report them as pending until
performed. For example, merge-triggered deployment and published-site verification
belong here, not in pre-merge acceptance. Pre-merge validation does not prove that
publication succeeded. See the [review guidance](https://github.com/takahirox/wonderswan-rust/blob/main/docs/review-guidelines.md#check-pre-merge-acceptance-and-post-merge-verification).
