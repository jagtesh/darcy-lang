#!/usr/bin/env python3
import json
from pathlib import Path

def escape_regex(items):
    return "|".join(sorted([item.replace("/", "\\/") for item in items], key=len, reverse=True))


def main():
    root = Path(__file__).resolve().parent
    defs = json.loads((root / "defs.json").read_text())

    keywords = defs["keywords"]
    types = defs["types"]
    builtins = defs["builtins"]
    operators = defs["operators"]

    grammar = {
        "name": "Darcy",
        "scopeName": defs["scope"],
        "patterns": [
            {"include": "#comments"},
            {"include": "#strings"},
            {"include": "#numbers"},
            {"include": "#keywords"},
            {"include": "#types"},
            {"include": "#builtins"},
            {"include": "#operators"},
        ],
        "repository": {
            "comments": {
                "patterns": [
                    {"name": "comment.line.semicolon.darcy", "match": ";.*$"},
                    {
                        "name": "comment.block.darcy",
                        "begin": "#\\|",
                        "end": "\\|#",
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
                            {"name": "constant.character.escape.darcy", "match": "\\\\."}
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
            "keywords": {
                "patterns": [
                    {
                        "name": "keyword.control.darcy",
                        "match": rf"\\b(?:{escape_regex(keywords)})\\b",
                    }
                ]
            },
            "types": {
                "patterns": [
                    {
                        "name": "storage.type.darcy",
                        "match": rf"\\b(?:{escape_regex(types)})\\b",
                    }
                ]
            },
            "builtins": {
                "patterns": [
                    {
                        "name": "support.function.builtin.darcy",
                        "match": rf"\\b(?:{escape_regex(builtins)})\\b",
                    }
                ]
            },
            "operators": {
                "patterns": [
                    {
                        "name": "keyword.operator.darcy",
                        "match": rf"(?:{escape_regex(operators)})",
                    }
                ]
            },
        },
    }

    grammar_json = json.dumps(grammar, indent=2)
    (root / "vscode" / "syntaxes" / "darcy.tmLanguage.json").write_text(grammar_json)
    (root / "zed" / "darcy.tmLanguage.json").write_text(grammar_json)


if __name__ == "__main__":
    main()
