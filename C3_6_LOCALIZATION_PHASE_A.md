# C3.6 phase A — external living-world localization

Status: implementation candidate; final C3.6 acceptance requires broader UI migration.
Platform: Windows x64 (owner decision, 2026-11-27). No Linux/WSL or hosted CI required. Local-only Git; no push.
Baseline: accepted Windows automated C3.5.1 candidate c494656eb4841254744f26b6224ea29f9af68bbe (tree d5475bb5265d9fc621bf30755920ba7f3f30679a).

## Implemented
- Runtime English/Indonesian flat JSON catalogs: apps/sealed_playable/locales/living_en.json and living_id.json.
- Stable semantic IDs for 13 messages: save/load/reset, campaign mismatch, legacy-save guidance, unrecordable intent, current help, background labels.
- Immutable once-per-process catalog, selected language -> English -> diagnostic ID fallback.
- Embedded catalogs remain the default, so existing standalone executables need no external assets to start.
- External packs selected via SEALED_LOCALE_DIR; existing absolute directory only. Optional English overrides augment embedded English; a supplied Indonesian dictionary replaces the embedded Indonesian map and falls back to English for missing keys.
- Whole override pair validated before publication; malformed, duplicate, empty, unknown keys, wrong placeholders, oversized or non-UTF-8 files, and forbidden control characters are rejected. Explicit diagnostic + embedded fallback on failure.
- Named parameters use {name}; {{ and }} escape braces. A translation can reorder or repeat parameters but cannot remove or invent them. Inserted argument data is never reparsed.
- No sealed_sim source, rules, state hash, save slot, dependency or lockfile changes.

## Usage (PowerShell)
    $env:SEALED_LOCALE_DIR = 'D:\path\to\your\locales'
    .\sealed_agency.exe
    # In the game: lang id, save, load, help
    Remove-Item Env:SEALED_LOCALE_DIR

Restart the application to load edited catalog files. An optional custom pack must use existing stable IDs. UTF-8 JSON, max 256 KiB per file and 8192 bytes per message.

## Translation context
- living.save.ok, living.load.ok, living.reset.ok: successful no-tick file/reset UI notices.
- living.save.error, living.load.error: include exact {error} parameter, a diagnostic string; translators may reposition it.
- living.save.foreign, living.load.foreign: revision mismatch guard; do not imply destructive migration.
- living.load.legacy: instructs player to use original legacy save slot, keep literal command --c09.
- living.intent.unrecordable: structural intent refusal before mutation; not ordinary tick-consuming rejection.
- living.help.c351: player help; preserve CLI command tokens and --c35.
- living.background.*: display names only; canonical enum values and parser tokens unchanged.

## Test contract
The catalog unit suite validates embedded parity, partial-language fallback, named placeholders, malformed dictionaries, reordered arguments, escaped literals, bounded reads and UTF-8. Executable integration tests launch actual sealed_agency with separate temporary save roots, valid and malformed external language packs, and compare checkpoint bytes across variants.
Windows cargo test / clippy output is recorded outside source under _c351_recovery_inputs/c36-windows-evidence.

## Remaining C3.6 work
Phase A is deliberately incremental. The older text(locale,en,id) sites, other render paths and authored dialog/description messages still need extraction; the older desktop catalog subsystem has not been converted to runtime overrides. Full context/export-import workflow, locale-aware plurals and grammar, full UI migration, and complete Windows release replay are subsequent acceptance work. No claim of full C3.6 completion. The historical C3.5.1 source and executable evidence remain preserved.
