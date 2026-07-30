# 0048 Repository Truth Reachability and State Projection Drift

Status: OPEN

Kind: owner-raised repository-integration, continuation-projection, and next-action integrity investigation

Priority: HIGH

Target: remote integration facts, default-branch reachability, repository technical truth, curated continuation freshness, PLAN/PROJECT_STATE responsibility, task handoff, and next-safe-action derivation behind the migrated repository-truth reconciliation decision

## Scope and authority

This topic follows the migrated `0040 Repository Truth Synchronization` decision and D33. It does not reopen the separation that GitHub owns pull-request draft, ready, check, base, and merge facts; Git commits, registered work-package records, audits, and evidence own candidate, review, admission, implementation, and completion truth; and `PROJECT_STATE.md` is a curated continuation projection that cannot override those owners.

It records a Project Owner concern that the three-layer truth model is directionally correct but the current projection protocol has already allowed a fresh session to observe an impossible current objective. The concern is not that every prose delay is a technical defect. It is that stale projection can repeat completed work, preserve a blocker whose release event has already occurred, miss content merged into the wrong base, or derive a next safe action from a pull-request flag without proving default-branch reachability and expected repository content.

This topic does not prescribe a write-capable post-merge workflow, recursive state-only pull request, remote-dependent local checker, state database, branch policy, metadata schema, task bot, or new gate. It does not authorize stale prose to change admission or completion truth. It does not block WP-300, the reviewed narrow Property Read path, or unrelated admitted work merely by existing. Codex owns the repository-grounded governance and evidence decision.

## Repository observations

- The migrated `0040` decision defines GitHub remote facts, repository technical/evidence truth, and curated continuation state as separate owners and requires fresh substantial sessions to reconcile them before relying on the recorded objective.
- `AGENTS.md` and `PROJECT_GOVERNANCE.md` assign maintenance of `PROJECT_STATE.md` and milestone progress in `PLAN.md` to AI agents, require continuous checkpointing, and state that the task after integration updates continuation state in its first checkpoint.
- The same governance rejects a write-capable post-merge workflow and a recursive state-only pull-request loop, so continuity currently depends on a later bounded task carrying the projection update.
- Current `PROJECT_STATE.md` still describes pull request #3 as a draft awaiting independent review and integration, and its Next Safe Actions begin by reviewing and integrating that already completed pull request.
- Current `PLAN.md` likewise describes the WP-300 correction as `pending`/`review-pending` because it still awaits independent review and integration.
- GitHub records pull request #3 as merged into `master` on 2026-07-30, and pull requests #4 through #10 subsequently completed repository changes on top of the default branch.
- Pull request #7 also reports `merged = true`, but its base was `agent/cleanup-complexity-risk` rather than `master`; its 0045 content did not become default-branch truth until pull request #9 restored it.
- Pull request #9 explicitly records that repair, showing that pull-request merge state alone is insufficient to establish expected default-branch content.
- Recent workspace-only pull requests intentionally excluded `PLAN.md` and `PROJECT_STATE.md` from their scope, so several successful task handoffs accumulated without repairing the older continuation projection.
- The workspace index on current `master` lists OPEN topics 0042 through 0047, while `PROJECT_STATE.md` still names only topics through 0041 as the primary current workspace range.
- The previous `0040` investigation recorded a case where `check-state` passed while a work-package check detected that the recorded continuation topology was impossible after remote integration.
- Offline validation cannot infer an unseen remote commit, but once a merge is present in fetched default-branch history, repository-local projection and next-action claims can be checked against that observed basis.

## Questions for investigation

### Truth domains and integration reachability

1. What exact facts are owned by GitHub pull-request metadata, and which require inspection of the fetched default-branch commit graph?
2. Is `merged = true` sufficient for any continuation claim when a pull request may target a non-default branch?
3. Which integration claim requires proving that the merge commit is an ancestor of the current default branch?
4. Which integration claim additionally requires proving that the expected paths or semantic changes are present on the default branch?
5. How are closed-unmerged, superseded, stacked, retargeted, merged-to-feature-branch, reverted, and default-branch-reachable outcomes distinguished?
6. Does a pull request retain one stable integration identity after its base changes?
7. What fact owns the relationship between a task branch, its intended base, its actual merge base, and the final default-branch descendant?
8. Can a later repair pull request be the true integration event for content whose original pull request already reports merged?
9. How is a reverted merge represented without rewriting the historical fact that the pull request once merged?
10. Which repository fact establishes that a remote validation status covers the merge revision rather than only the pull-request head?
11. Must default-branch reachability be established before a dependent task begins, even when the pull request UI reports merged?
12. What evidence distinguishes integration of the intended semantic content from integration of a branch name or pull-request number?

### Continuation basis and freshness identity

13. What exact observed default-branch commit or generation is the basis of one `PROJECT_STATE.md` projection?
14. Does current continuation state carry a machine-readable basis identity, or only prose references to earlier commits and pull requests?
15. How can a fresh agent determine whether the projection predates one, ten, or one hundred default-branch commits?
16. Is commit distance meaningful, or should freshness be based on whether intervening changes affect the objective, blockers, or next safe action?
17. Which changes require immediate continuation refresh: merge, review completion, admission, implementation, evidence, workspace decision, risk reclassification, or milestone transition?
18. Which changes may safely leave continuation prose stale because they are disjoint from the executable objective?
19. How is the last reconciled remote observation distinguished from the repository commit that contains the projection text?
20. Can a projection truthfully describe conditional pre-merge and post-merge next actions without becoming invalid after merge?
21. What information must survive when the session is offline and cannot verify whether the conditional transition occurred?
22. When remote access returns, what exact reconciliation step converts conditional state into one current next action?
23. How is an unverified remote assumption prevented from releasing source admission?
24. How is a verified merge prevented from remaining an artificial blocker after the default branch is fetched?

### Dangerous drift versus harmless staleness

25. What makes stale continuation text a dangerous defect rather than ordinary historical lag?
26. Is drift dangerous when it changes the next executable objective, blocker set, source-admission boundary, milestone status, or required evidence?
27. Is a stale list of already migrated workspace topics dangerous when the active implementation objective remains correct?
28. Can a stale pull-request reference be harmless if both conditional next actions still lead to the same safe repository transition?
29. What is the minimum counterexample proving that a fresh agent following `PROJECT_STATE.md` would perform completed, invalid, or unsafe work?
30. Does the current instruction to review and integrate PR #3 satisfy that counterexample?
31. Does retaining `pending/review-pending` after the correction merge create only reporting drift, or does it preserve a false WP-300 source blocker?
32. Which stale statements can be repaired with the next task, and which must stop the current task before further repository changes?
33. How is severity affected when a stale projection understates progress versus overstates admission or completion?
34. Should an overclaim and an underclaim have different blocking behavior?
35. Can a conservative stale blocker be tolerated indefinitely, or does repeated completed work become a governance failure?
36. Which risk state should R-011 hold after a concrete impossible objective is observed?

### PROJECT_STATE and PLAN responsibility

37. Which volatile execution facts belong only in `PROJECT_STATE.md` rather than `PLAN.md`?
38. Should `PLAN.md` name a specific transient pull request as the reason a tranche is pending?
39. Which milestone and package status belongs in `PLAN.md`, and which exact task handoff belongs only in continuation state?
40. How is duplication detected when both files repeat the same PR, review, blocker, or next action?
41. If the two projections disagree, which is repaired first and which authoritative owners determine the result?
42. Can `PLAN.md` remain stable while `PROJECT_STATE.md` advances through several pull-request handoffs within one milestone?
43. What repository event justifies changing milestone status independently of continuation state?
44. Should OPEN workspace topics be projected into `PLAN.md`, `PROJECT_STATE.md`, both, or neither when they are disjoint from the critical path?
45. Which state file owns the exact highest completed vertical tranche?
46. Which state file owns the currently selected next bounded task?
47. Can one structured source generate both projections without becoming a new technical authority?
48. What evidence demonstrates that the current responsibility split reduces rather than duplicates write obligations?

### Task handoff and update ownership

49. Which bounded task is responsible for the first post-integration continuation update?
50. What happens when the next task intentionally changes only workspace files and declares `PLAN.md` and `PROJECT_STATE.md` outside scope?
51. Can continuation repair travel with that task without contaminating its semantic review boundary?
52. When is a state update part of the truth recorded by the task, and when is it unrelated work?
53. Does every repository-changing task need to reconcile continuation before writing, or only tasks whose next safe action depends on remote integration?
54. Can several disjoint workspace topics be opened while the executable objective remains unchanged without updating PLAN or state each time?
55. What prevents a sequence of individually valid narrow PR scopes from collectively leaving continuation unusable?
56. Who owns recovery when the expected post-merge checkpoint was skipped by multiple tasks?
57. Should a task handoff state the observed default-branch basis even when it makes no continuation-file edit?
58. What evidence proves that the next agent performed reconciliation rather than merely read stale prose?
59. How is a task that starts from an outdated default branch detected before it creates another projection?
60. Which task transitions require a continuation update in the same commit, the same pull request, or only the next default-branch descendant?

### Stacked, retargeted, and repaired pull requests

61. What exact reconciliation rule applies to a pull request whose base is another task branch?
62. When its predecessor merges, must the dependent PR be retargeted to `master` before it can count as default-branch handoff?
63. If the dependent PR is merged into the predecessor branch, what state should the workspace topic, task, and continuation projection report?
64. Does a merge into a feature branch complete review, integration, both, or neither?
65. Which repository check confirms that the dependent topic actually appears in current `master`?
66. How are duplicate or superseded PRs represented so a fresh agent does not follow the wrong handoff?
67. Can one task have multiple remote PR artifacts while retaining one canonical integration event?
68. How does a repair PR preserve the history of the original mistake without making both appear independently completed on the default branch?
69. Which facts from the PR #7/#9 sequence should become a reusable regression scenario?
70. Does the current governance provide enough guidance to avoid repeating a stacked merge into a non-default base?

### Next-safe-action derivation

71. Which authoritative facts are inputs to computing the next safe action?
72. Is the next action stored as prose, derived from work-package state, or both?
73. What proves that every prerequisite named by the next action is still unsatisfied?
74. What proves that every release event already present in repository history is reflected in the next action?
75. Can the registered work-package DAG determine the next tranche while pull-request state only determines whether its admission base is available?
76. How is an exact `admission_base_ref` selected after multiple disjoint workspace merges advance `master` beyond the original correction merge?
77. Must the admission base be the first merge satisfying the prerequisite or the current reviewed default-branch descendant?
78. What revalidation is needed when disjoint commits occur between prerequisite integration and admission checkpoint creation?
79. Can a stale continuation objective cause the correct work-package checker to be invoked in the wrong mode?
80. What local evidence can reject a next action that references an already-merged PR or already-present path?
81. What remote evidence remains indispensable before a dependent source transition?
82. Which current next action is justified now that PR #3 and later workspace PRs are present on `master`?

### Checker and offline boundary

83. Which stale-projection invariants can be checked entirely from the local fetched repository?
84. Can a checker verify that a referenced commit is an ancestor of HEAD and that expected files exist without contacting GitHub?
85. Which PR states cannot be validated offline and must remain explicitly last-observed facts?
86. How should local validation treat a pull-request URL or number whose current remote state is unavailable?
87. Can the projection record an observed remote state plus observation time or basis without treating time alone as freshness authority?
88. What impossible objective can be detected by comparing current repository paths, ancestry, work-package records, and next-action prose?
89. Would such detection protect a distinct falsifiable invariant not already covered by the registered work-package checker?
90. How is a lightweight check prevented from becoming a remote-dependent or prose-parsing support system that blocks product work?
91. Can checker coverage remain narrow to admission-critical or completion-critical references rather than every narrative sentence?
92. What negative fixture would reproduce PR #3 being merged while the objective still says to merge it?
93. What negative fixture would reproduce PR #7 being merged to a feature branch while expected content is absent from `master`?
94. What evidence is sufficient to decide that no new checker is needed and disciplined reconciliation is enough?

### Current repair and maturity claim

95. Which current statements in `PROJECT_STATE.md` and `PLAN.md` are now factually stale?
96. Which of those statements affect only narration, and which affect WP-300 admission or the next executable action?
97. What is the current default-branch basis after PR #10?
98. Which PR #3 correction facts, review facts, and integration facts should be recorded durably?
99. Which later PRs changed only workspace investigation state and therefore do not alter WP-300 technical admission truth?
100. Does the exact five-file pre-source checkpoint remain the correct next WP-300 transition after the intervening disjoint merges?
101. What review or revalidation must bind the current default-branch descendant before that checkpoint?
102. Should the current stale projection be repaired inside the decision/migration packet for this topic or as the first checkpoint of the next executable task?
103. What continuation maturity claim is justified while fresh-session reconciliation remains behavioral rather than mechanically enforced?
104. Can R-011 return to MONITOR only after both the current projection and the recurring mechanism gap are closed?
105. Which concrete recurrence would reopen the topic after migration?

### Existing decision intersection

106. Which questions are already fully resolved by `0040`, D33, `AGENTS.md`, `PROJECT_GOVERNANCE.md`, work-package checks, and remote task governance?
107. Which questions are normal execution discipline rather than missing governance or checker contracts?
108. Which questions reveal a genuinely unclosed default-branch reachability, projection freshness, responsibility, or next-action contract?
109. Does any finding require changing technical architecture, or is the scope confined to governance, continuation, planning projection, and evidence tooling?
110. Which findings can be corrected without reopening the immutable WP-300 semantic review?
111. Which findings require revalidating only the admission base and next-state transition?
112. Would any additional requirement, state field, fixture, checker, workflow, or handoff rule protect a distinct falsifiable claim not already owned by existing artifacts?

## Constraints

- Preserve the separation among GitHub remote facts, repository technical/evidence truth, and curated continuation projections.
- Preserve offline source correctness; remote availability must not become necessary for local architecture or implementation correctness.
- Do not let stale `PROJECT_STATE.md` or `PLAN.md` override registered admission, completion, implementation, or commit-graph truth.
- Do not infer default-branch integration from `merged = true` without considering the actual base, reachability, and expected repository content.
- Do not prescribe a write-capable post-merge workflow, recursive state-only PR, state database, checker design, metadata schema, or branch policy before Codex classifies the evidence.
- Do not require every harmless narrative lag to block unrelated work.
- Do not allow a projection known to direct a fresh agent toward completed, impossible, or unsafe work to remain merely MONITOR without an explicit disposition.
- Preserve exact candidate, review-attestation, admission-base, implementation, and evidence topology.
- Do not reopen the WP-300 semantic candidate merely because disjoint workspace commits advanced the default branch.
- Do not add a blocking gate unless it protects a distinct falsifiable claim not already covered by existing ancestry, work-package, evidence, mainline, or task checks.
- Do not block WP-300 or unrelated admitted work merely because this topic is OPEN; Codex must identify the exact intersection.

## Expected decision output

Codex should:

1. classify which observed drift is harmless narration, dangerous next-action drift, admission-impacting drift, or already-resolved history;
2. define the exact integration fact needed for dependent work, including pull-request state, actual base, default-branch reachability, expected content, and merge-revision validation;
3. define the responsibility and freshness boundary among `PROJECT_STATE.md`, `PLAN.md`, work-package records, Git history, and remote observations;
4. determine how a fresh session proves that its current objective and next safe action remain possible and unsatisfied;
5. determine whether current reconciliation discipline, state structure, fixtures, checkers, or handoff rules are sufficient without introducing recursive or remote-dependent machinery;
6. identify the exact current correction needed for the stale PR #3/WP-300 projection and the justified admission-base next step after intervening disjoint merges;
7. identify any unsupported integration, milestone, admission, completion, workspace, or next-action claim in current repository projections; and
8. migrate only conclusions supported by repository evidence.