# C3.6 — Windows runtime localization

## Scope and provenance
The standalone Rust/Bevy game targets Windows x64 only by owner decision. C3.5.1 baseline c494656eb4841254744f26b6224ea29f9af68bbe; phase A localization commit 3ebcb991b78809798e7d4a63a0a3de89871f9f43. No remote writes or main changes. Pure simulation source, rules revision 26, checkpoints, lockfiles and dependencies are unchanged.

## Presentation coverage
- 552 living-world message IDs per language (470 referenced IDs, 82 retained compatibility fragments marked inactive in context). All previous paired text/tr calls and literal goal/outcome/rejection/rank/reason tuples were extracted from the living ASCII and pixel views.
- 93 newly parameterized display templates; static translated fragments were inlined into complete sentences where possible. The English/Indonesian pair includes previously English-only combat/cultivation notices, empty journals and the pixel window title. English singular/other and Indonesian invariant noun forms are supported for cargo recovery, with correct English fallback.
- 203 legacy UI message IDs per language (base C0.4, C0.5–C0.9 overlays and nine new C3.6 legacy strings). Legacy ASCII and native Bevy UI load the same runtime dictionary family.
- Immutable catalogs load once per process. Tab in pixel mode / lang en or lang id in living ASCII change presentation only; stale translated notices are cleared on a real language change. Named arguments allow reordering and repetition, not removal or invention; inserted values are not reparsed. Runtime output expansion is bounded to 64 KiB.
- Explicit absolute SEALED_LOCALE_DIR takes precedence; otherwise read the locales directory next to the executable. Never load from arbitrary working directory. No pack is required (embedded fallback).
- A malformed override pair within a dictionary family is rejected entirely with a diagnostic. Missing selected-language keys fall back to English; missing English overrides fall back to embedded English. A supplied partial Indonesian file intentionally replaces the Indonesian map; absent keys fall back to English.
- Caps: 256 KiB per runtime JSON dictionary, 8192 UTF-8 bytes per message; duplicate, unknown, empty, invalid UTF-8, wrong-parameter and control-character errors rejected. English and Indonesian shipped catalog parity verified.

## Translator workflow
From apps/sealed_playable/tools:
    python locale_pack.py export --out D:\translation\sealed-c36
    python locale_pack.py check --pack D:\translation\sealed-c36
    python locale_pack.py import --kind living --language id --input D:\translation\downloaded.json --out D:\translation\custom
The export is flat UTF-8 JSON suitable for a Crowdin JSON workflow. Upload the English dictionaries as source files, provide the context sidecar to translators, and import translated JSON after validation. No Crowdin account/service connection is required or claimed. The context file identifies inactive retained keys, original function/surface, and dynamic parameter meanings. Never change a message ID or canonical command token. The tool refuses overwrite unless --replace is explicit.
Set SEALED_LOCALE_DIR to the exported/custom directory or copy the four runtime dictionaries into locales beside the exe. Restart after editing. The context sidecars are for authors and not consumed by the engine.

## Acceptance gates (Windows)
Run pinned Rust 1.100.0 locked cargo fmt/check/test/clippy for core, headless and desktop, release builds, actual executable replay against the C3.5.1 checkpoint hashes, GUI startup/close, Tab projection test, translator tool tests, reproducible patch and exact Git archive verification. Record actual results outside source; no tests or review claims are implied by this design document.

## Explicit limits
The language selector is EN/ID, not an arbitrary-language or CLDR engine. Plural selection is implemented for the cargo recovery sentence, not universal grammatical inflection. Proper names, canonical command syntax, debug enum/diagnostic identifiers, raw OS errors and developer diagnostics intentionally remain literal. The historical UI still has some layout-oriented concatenation/formatting, while the main living UI uses named templates. Locale switches clear prior notices rather than reconstructing past observation commands. Runtime packs load at startup, not hot reload; no live Crowdin integration. Human playtest and independent audit remain separate from automated verification. C3.7 data-driven content and registry changes are not part of this presentation-only milestone.
