# C2.6 — Readable investigation

Small ASCII playability improvement on top of C2.5. `crimelog` lists only incidents the player caused, suffered, or personally knows, with separate labels for direct observation, firsthand accounts and allegations. Cargo theft is never described as a zero-damage blow. The command is read-only, localized in English and Indonesian, and refuses to upgrade revision-15 campaigns. `bodies` remains a separate list of physically discovered corpses: seeing one does not identify the killer. No simulation/checkpoint semantics or C0.1 kernel files changed; C2.5 revision 16 remains intact.

Regression tests run the complete public ASCII courier ambush, corpse theft, both languages, and save/load, then assert `crimelog` cannot advance time or alter checkpoint bytes. The legacy C2.4 view also remains unchanged. No human or Windows playtest.

Verification actually run (offline Linux): simulation 276 passed, 1 ignored; playable 84 passed; both workspaces' Rustfmt checks and Clippy all-target checks with `-D warnings` clean. The same script in the C2.5 report can append `crimelog` to inspect the result. No push or merge.

Remaining limitation: this is a read-only clarity increment, not a new investigative action. Next, give the player a physically delivered report to the watch and a meaningful response to corpse evidence, under a new checkpoint revision if simulation semantics change.
