#!/usr/bin/env python3
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DEFS_PATH = ROOT / "extensions" / "defs.json"
VSCODE_GRAMMAR_PATH = ROOT / "extensions" / "vscode" / "syntaxes" / "darcy.tmLanguage.json"
ZED_GRAMMAR_PATH = ROOT / "extensions" / "zed" / "darcy.tmLanguage.json"
TYPE_ALIASES_PATH = ROOT / "dslc" / "src" / "type_aliases.rs"

IDENT_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_-]*$")
TYPE_ALIAS_RE = re.compile(r'\("([^"]+)",\s*"([^"]+)"\)')


def load_defs() -> dict:
    defs = json.loads(DEFS_PATH.read_text())
    alias_types = load_type_aliases()
    defs["types"] = sorted(set(defs["types"]).union(alias_types))
    return defs


def load_type_aliases() -> set[str]:
    text = TYPE_ALIASES_PATH.read_text()
    out: set[str] = set()
    for alias, resolved in TYPE_ALIAS_RE.findall(text):
        out.add(alias)
        out.add(resolved)
    return out


def token_pattern(token: str) -> str:
    escaped = re.escape(token)
    if IDENT_RE.match(token):
        return rf"\\b{escaped}\\b"
    return escaped


def alternatives(tokens: list[str], *, with_word_boundaries: bool) -> str:
    uniq = sorted(set(tokens), key=lambda s: (-len(s), s))
    parts = []
    for tok in uniq:
        if with_word_boundaries:
            parts.append(token_pattern(tok))
        else:
            parts.append(re.escape(tok))
    return "(?:" + "|".join(parts) + ")"


def build_grammar(defs: dict) -> dict:
    keyword_pattern = alternatives(defs["keywords"], with_word_boundaries=True)
    type_pattern = alternatives(defs["types"], with_word_boundaries=True)
    builtin_pattern = alternatives(defs["builtins"], with_word_boundaries=False)
    operator_pattern = alternatives(defs["operators"], with_word_boundaries=False)

    return {
        "name": "Darcy",
        "scopeName": defs["scope"],
        "patterns": [
            {"include": "#comments"},
            {"include": "#strings"},
            {"include": "#numbers"},
            {"include": "#symbol_literals"},
            {"include": "#typed_members"},
            {"include": "#keywords"},
            {"include": "#types"},
            {"include": "#builtins"},
            {"include": "#operators"},
        ],
        "repository": {
            "comments": {
                "patterns": [
                    {
                        "name": "comment.line.semicolon.darcy",
                        "match": rf"{re.escape(defs['comment_line'])}.*$",
                    },
                    {
                        "name": "comment.block.darcy",
                        "begin": re.escape(defs["comment_block"][0]),
                        "end": re.escape(defs["comment_block"][1]),
                    },
                ]
            },
            "strings": {
                "patterns": [
                    {
                        "name": "string.quoted.double.darcy",
                        "begin": '"',
                        "end": '"',
                        "patterns": [
                            {
                                "name": "constant.character.escape.darcy",
                                "match": r"\\.",
                            }
                        ],
                    }
                ]
            },
            "numbers": {
                "patterns": [
                    {
                        "name": "constant.numeric.float.darcy",
                        "match": r"-?\\b\\d+\\.\\d+(?:[eE][+-]?\\d+)?\\b",
                    },
                    {
                        "name": "constant.numeric.integer.darcy",
                        "match": r"-?\\b\\d+\\b",
                    },
                ]
            },
            "symbol_literals": {
                "patterns": [
                    {
                        "name": "constant.other.symbol.darcy",
                        "match": r"(?:(?<=^)|(?<=[\s\[\]{}(),]))::?[^\s\[\]{}()\",;]+",
                    }
                ]
            },
            "typed_members": {
                "patterns": [
                    {
                        "name": "entity.name.function.darcy",
                        "match": r"\b[A-Za-z_][A-Za-z0-9_?!-]*:[A-Za-z_][A-Za-z0-9_?!-]*\b",
                    }
                ]
            },
            "keywords": {
                "patterns": [
                    {
                        "name": "keyword.control.darcy",
                        "match": keyword_pattern,
                    }
                ]
            },
            "types": {
                "patterns": [
                    {
                        "name": "storage.type.darcy",
                        "match": type_pattern,
                    }
                ]
            },
            "builtins": {
                "patterns": [
                    {
                        "name": "support.function.builtin.darcy",
                        "match": builtin_pattern,
                    }
                ]
            },
            "operators": {
                "patterns": [
                    {
                        "name": "keyword.operator.darcy",
                        "match": operator_pattern,
                    }
                ]
            },
        },
    }


def write_if_changed(path: Path, data: dict) -> bool:
    rendered = json.dumps(data, indent=2) + "\n"
    old = path.read_text() if path.exists() else ""
    if old == rendered:
        return False
    path.write_text(rendered)
    return True


def main() -> int:
    defs = load_defs()
    grammar = build_grammar(defs)
    changed = False
    changed |= write_if_changed(VSCODE_GRAMMAR_PATH, grammar)
    changed |= write_if_changed(ZED_GRAMMAR_PATH, grammar)
    print("updated grammars" if changed else "grammars already up to date")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
