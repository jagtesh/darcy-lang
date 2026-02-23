#!/usr/bin/env python3
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DEFS_PATH = ROOT / "extensions" / "defs.json"
VSCODE_GRAMMAR_PATH = ROOT / "extensions" / "vscode" / "syntaxes" / "darcy.tmLanguage.json"
ZED_GRAMMAR_PATH = ROOT / "extensions" / "zed" / "darcy.tmLanguage.json"

IDENT_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_-]*$")


def load_defs() -> dict:
    return json.loads(DEFS_PATH.read_text())


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
            {"include": "#keyword_literals"},
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
            "keyword_literals": {
                "patterns": [
                    {
                        "name": "constant.other.keyword.darcy",
                        "match": r":[^\s\[\]{}()\";]+",
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
