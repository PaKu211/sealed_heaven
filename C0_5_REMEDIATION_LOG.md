# C0.5 independent findings and remediation

No final acceptance yet. Initial verified implementation:
`ad0410cf469eea29df47baa84de4da4e7f45fd72` (pushed; remote SHA matched).

| Finding | Evidence | Remediation / closure state |
| --- | --- | --- |
| CORE-01, medium | Independent checksummed-save probes admit impossible remote Talk success, false occupancy rejection and forbidden occupied-cell movement | Full bounded NPC-history replay rejects the three forged variants; only the authentic C03 `next_event_id == u64::MAX` cursor exception is permitted, preserving atomic capacity failure. New in-repo regression and external public-API probe pass. Independent post-fix re-review remains open. |
| CLIENT-01, high | Actual native clock path receives virtual250ms for a real1s frame; regression failed before fix (tick0 instead of2) | `Time<Real>` for elapsed/frame metrics; native1s/2s test passes ticks2/4 and1000ms. Independently closed. |
| CLIENT-02, medium | Same-frame Tab/E/N submits Talk instead of Gather; regression failed before fix | Fold pending button/key control state in order; batched target/toggle/language/mixed-input regression passes. Independently closed. |
| CLIENT-03, low | ID21 text stays at entrance | Child label follows the actual transform hierarchy after movement/reset. Independently closed. |
| CLIENT-04, medium | Native clicks landing on button labels fail to activate parent Interact/Step controls; clock could step without intended action | `FocusPolicy::Pass` on child labels and ordered input/duplicate handling were added in the shared working tree. Native/source-independent review of this addition remains outstanding; final native driver uses keyboard E/N and does not certify mouse-click labels. |

Core restore work must be bounded. The slice has an explicit4096-tick save/drive
horizon (about34m08s at nominal2 Hz), in addition to4096 command identities. This
prevents huge-tick saves from triggering unbounded replay. No C01–C04 contract,
golden vector or old simulation behavior changes. C05 revision1 is still an
unaccepted development format.

First native attempt `c05-native-gameplay-01` stopped before input. Attempt 02 focused
the window and stepped, but the action log was empty: only the pre-action hint
"Too far" was visible, so the rejection sentence was absent. Attempt 06, on the
later release binary, recorded `#1 · t1 · Rejected: Target is not within one cell`
and completed the gameplay script. Evidence:
`C:\Users\PC_4\AppData\Local\Temp\opencode\c05-native-gameplay-09`. The current
driver records a queued-intent capture and checks exactly one N step consumes it;
it uses keyboard E/N for the action loop. The latest run has 31 F12 pairs and
passed all in-driver assertions. This is gameplay evidence, not final acceptance.

Client reviewer independently ran all4 native-binary unit tests in debug/release
and recorded source-only PASS in `C0_5_CLIENT_INDEPENDENT_REVIEW.md`. This is not
an interactive or performance verdict.

## Continuation verification (2026-11-18)

Full `tools/c05_verify.ps1` passed after remediation: root 100 passed / 1 explicit
maximum-horizon latency test ignored, 5 benchmark tests and 2 doctests in both
profiles; app 41 tests per profile; fmt/check/strict Clippy, canonical packers,
and debug/release native builds passed. Native run succeeded at
`C:\Users\PC_4\AppData\Local\Temp\opencode\c05-native-gameplay-09` with 31
F12 pairs. Final matrix is in `C0_5_PERFORMANCE_BASELINE.md`; raw evidence is in
`c05-performance-final-03`. These do not replace the outstanding independent core
re-review/final acceptance.

Post-remediation external public-API probe was also executed locally in debug and
release. Its 8/8 tests pass, including all three original forged-history cases,
positive controls, and restoration of an authentic NPC candidate with its event
cursor set to `u64::MAX`, followed by atomic NPC-capacity failure. Initial replay
validation rejected that cursor-only candidate; the validator now allows only the
exhausted cursor exception while still comparing the complete non-cursor NPC
history/digest. This is corroborating regression evidence, **not** the required
independent re-review; the original reviewer has not reviewed this final fix.
