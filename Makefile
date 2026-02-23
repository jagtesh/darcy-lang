BENCH_ITERS ?= 10000
BENCH_BASELINE ?= bench/baseline.json
BENCH_CANDIDATE ?= bench/latest.json
BENCH_HISTORY ?= bench/history
BENCH_LABEL ?= main
BENCH_MAX_PCT ?= 5

.PHONY: bench-typecheck-run bench-check bench-accept vscode-deps vscode-package grammars grammars-check fmt test precommit hooks-install

bench-typecheck-run:
	cargo run -p dslc --release --bin bench_typecheck -- --iters $(BENCH_ITERS) --save $(BENCH_CANDIDATE) --save-dir $(BENCH_HISTORY) --label $(BENCH_LABEL)

bench-check: bench-typecheck-run
	cargo run -p dslc --bin bench_typecheck -- compare --baseline $(BENCH_BASELINE) --candidate $(BENCH_CANDIDATE) --max-regression-pct $(BENCH_MAX_PCT)

bench-accept:
	cargo run -p dslc --bin bench_typecheck -- update --baseline $(BENCH_BASELINE) --candidate $(BENCH_CANDIDATE)

vscode-deps:
	cd extensions/vscode && npm install

vscode-package:
	cd extensions/vscode && npx @vscode/vsce package

grammars:
	python3 extensions/gen_grammars.py

grammars-check:
	python3 extensions/gen_grammars.py
	git diff --exit-code -- extensions/vscode/syntaxes/darcy.tmLanguage.json extensions/zed/darcy.tmLanguage.json

fmt:
	cargo fmt --all

test:
	cargo test --workspace

precommit: fmt grammars-check test

hooks-install:
	git config core.hooksPath .githooks
	chmod +x .githooks/pre-commit
