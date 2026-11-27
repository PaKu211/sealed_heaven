#!/usr/bin/env python3
"""Export/import Sealed Heaven flat JSON locale packs (Crowdin-compatible layout).

No network or Crowdin credentials are required. This validates transport files;
the game's Rust loader independently validates every runtime override.
"""
from __future__ import annotations
import argparse
import json
from pathlib import Path
import re
import sys
import unicodedata

SOURCE = Path(__file__).resolve().parents[1] / "locales"
MAX_PACK = 256 * 1024
MAX_MESSAGE = 8192
KINDS = ("living", "ui")
LANGS = ("en", "id")
UI_STEMS = ("", "c05_", "c06_", "c07_", "c08_", "c09_", "c36_ui_")


def checked_pairs(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate key: {key}")
        result[key] = value
    return result


def read_json(path: Path):
    with path.open("rb") as file:
        data = file.read(MAX_PACK + 1)
    if len(data) > MAX_PACK:
        raise ValueError(f"file exceeds {MAX_PACK} bytes: {path}")
    return json.loads(data.decode("utf-8"), object_pairs_hook=checked_pairs)


def check_message(key, value):
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{key}: expected nonempty string")
    if len(value.encode("utf-8")) > MAX_MESSAGE:
        raise ValueError(f"{key}: message exceeds {MAX_MESSAGE} bytes")
    if any(unicodedata.category(c) == "Cc" and c not in "\n\t" for c in value):
        raise ValueError(f"{key}: forbidden control character")


def parameters(value):
    """Same literal-brace and named-parameter contract as Message::parse."""
    found = set()
    i = 0
    while i < len(value):
        c = value[i]
        if c in "{}" and value[i:i+2] == c+c:
            i += 2
        elif c == "{":
            end = value.find("}", i+1)
            if end < 0 or not re.fullmatch(r"[a-z_][a-z0-9_]*", value[i+1:end]):
                raise ValueError("invalid or unclosed parameter")
            found.add(value[i+1:end])
            i = end+1
        elif c == "}":
            raise ValueError("unescaped closing brace")
        else:
            i += 1
    return found


def source_catalog(kind, language):
    if kind == "living":
        result = read_json(SOURCE / f"living_{language}.json")
    else:
        result = {}
        for stem in UI_STEMS:
            overlay = read_json(SOURCE / f"{stem}{language}.json")
            if not isinstance(overlay, dict):
                raise ValueError(f"invalid dictionary: {stem}{language}")
            duplicates = result.keys() & overlay.keys()
            if duplicates:
                raise ValueError(f"shadowed UI IDs: {sorted(duplicates)}")
            result.update(overlay)
    if not isinstance(result, dict):
        raise ValueError("expected flat JSON dictionary")
    for key, value in result.items():
        check_message(key, value)
        if kind == "living":
            parameters(value)
    return result


def validate(kind, candidate, baseline, *, complete=False):
    if not isinstance(candidate, dict):
        raise ValueError("expected flat JSON object")
    if extra := candidate.keys() - baseline.keys():
        raise ValueError(f"unknown {kind} IDs: {sorted(extra)}")
    if complete and candidate.keys() != baseline.keys():
        raise ValueError(f"missing {kind} IDs: {sorted(baseline.keys()-candidate.keys())}")
    for key, value in candidate.items():
        check_message(key, value)
        if kind == "living" and parameters(value) != parameters(baseline[key]):
            raise ValueError(f"{key}: parameter contract mismatch")
    return len(candidate), len(baseline) - len(candidate)


def base_catalogs():
    result = {}
    for kind in KINDS:
        en = source_catalog(kind, "en")
        id_ = source_catalog(kind, "id")
        validate(kind, id_, en, complete=True)
        result[kind] = {"en": en, "id": id_}
    return result


def encode(value):
    return (json.dumps(value, indent=2, ensure_ascii=False, sort_keys=True)+"\n").encode("utf-8")


def write_new(path: Path, data: bytes, replace=False):
    if len(data)>MAX_PACK and path.name.endswith(("_en.json","_id.json")):
        raise ValueError("serialized dictionary exceeds runtime byte limit")
    path.parent.mkdir(parents=True, exist_ok=True)
    # Avoid truncating a previous translation before the new file is ready.
    if not replace:
        with path.open("xb") as file:
            file.write(data)
    else:
        import os, tempfile
        with tempfile.NamedTemporaryFile(dir=path.parent, prefix=".locale-", delete=False) as file:
            temporary = Path(file.name)
            try:
                file.write(data)
                file.flush()
                os.fsync(file.fileno())
            except BaseException:
                file.close()
                temporary.unlink(missing_ok=True)
                raise
        try:
            temporary.replace(path)
        finally:
            temporary.unlink(missing_ok=True)


def export_pack(out, catalogs):
    out.mkdir(parents=True, exist_ok=False)
    for kind, translations in catalogs.items():
        for language, values in translations.items():
            write_new(out / f"{kind}_{language}.json", encode(values))
    context = json.loads((SOURCE/"living_context.json").read_text(encoding="utf-8"),
                         object_pairs_hook=checked_pairs)
    assert context.keys() == catalogs["living"]["en"].keys()
    write_new(out/"living_context.json", encode(context))
    write_new(out/"ui_context.json", encode({
        key: {"context": key.replace(".", " / "), "active": True}
        for key in catalogs["ui"]["en"]
    }))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="action", required=True)
    p = sub.add_parser("export", help="create a new full translation directory")
    p.add_argument("--out", type=Path, required=True)
    p = sub.add_parser("check", help="validate a complete or partial runtime pack")
    p.add_argument("--pack", type=Path, required=True)
    p = sub.add_parser("import", help="validate downloaded flat JSON and publish one file")
    p.add_argument("--kind", choices=KINDS, required=True)
    p.add_argument("--language", choices=LANGS, required=True)
    p.add_argument("--input", type=Path, required=True)
    p.add_argument("--out", type=Path, required=True, help="target pack directory")
    p.add_argument("--replace", action="store_true", help="explicitly replace existing translation")
    args = parser.parse_args()
    catalogs = base_catalogs()
    if args.action == "export":
        export_pack(args.out, catalogs)
        print(f"Exported living={len(catalogs['living']['en'])}, ui={len(catalogs['ui']['en'])}")
    elif args.action == "check":
        count = 0
        for kind in KINDS:
            for language in LANGS:
                path = args.pack/f"{kind}_{language}.json"
                if path.exists():
                    present, missing = validate(kind, read_json(path), catalogs[kind]["en"])
                    print(f"{path.name}: PASS {present} provided, {missing} fallback")
                    count += 1
        if not count:
            raise ValueError("no runtime dictionaries found")
    else:
        candidate = read_json(args.input)
        present, missing = validate(args.kind, candidate, catalogs[args.kind]["en"])
        write_new(args.out/f"{args.kind}_{args.language}.json", encode(candidate), args.replace)
        print(f"Imported {present} IDs; {missing} fallback")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, UnicodeError) as error:
        print(f"Locale pack rejected: {error}", file=sys.stderr)
        sys.exit(2)
