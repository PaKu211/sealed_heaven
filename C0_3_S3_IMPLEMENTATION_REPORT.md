# C0.3 S3 implementation report

Branch `feat/c03-s3-navigation`, based on verified/pushed S2 `509d9c0`.

Implemented integer-cost route search with whole-path lexicographic tie breaking,
separate N/E/S/W grid BFS, checked cost sums, authoritative actor travel, stored
path/index continuation, and residual edge travel for enabled or disabled actors.
The approved fixture graph and its costs are unchanged; algorithm-only tests use
separate graphs for weighting, ties, and overflow. No crate was added.

Rest/Return Home now plan routes and complete with empty transactions. Work holds
the lowest tool/workstation without consuming durable quantities. Haul reserves
its inventory unit, travels home, and atomically credits existing/new home stock.
Unreachable work uses the same ordered release/failure mechanism as invalidation.

## Verification

75 unit tests and 2 doctests pass in both debug and release. Formatting,
locked/offline check and strict Clippy pass. Existing C0.1/C0.2 goldens and the
two-claimant oracle pass unchanged. M0-T07 reproduces the published initial blob
`bd803a2c...`, all four transfers/recipe outputs, and tick-10 later-leg rollback
to identical tick-9 bytes. Restore comparisons cover grid progress, edge start,
residual edge state and each scripted transfer's pending completion.

One initial M0-T07 test used interval 4 and failed its published hash. Corrected
the fixture to interval 100 as recorded by the prior v1.3 final audit; no oracle
or production codec was changed to make it pass.

## Review attention: residual timing

Acceptance §14 explicitly saves a just-cancelled actor still on edge 1 at cost 0,
then arrives on the next drive. The broad specification §8 sentence can instead
be read as spending residual cost in the cancellation boundary itself. The code
follows the concrete acceptance sequence: residual eligibility is taken from
canonical boundary-start state, with no hidden persistent flag. Save/restore
therefore produces the same next-boundary travel. This interpretation requires
independent review; neither document has been silently rewritten.

Status: verified implementation evidence, not independent acceptance. S4's
20-actor suite and combined acceptance remain to be completed.
