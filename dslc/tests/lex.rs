use dslc::{lex, TokKind};

#[test]
fn lexes_basic_form() {
    let src = "(defn add [a:i32 b:i32] (+ a b))";
    let toks = lex(src).expect("lex ok");
    let kinds: Vec<TokKind> = toks.into_iter().map(|t| t.kind).collect();
    let expected = vec![
        TokKind::LParen,
        TokKind::Sym("defn".to_string()),
        TokKind::Sym("add".to_string()),
        TokKind::LBrack,
        TokKind::Sym("a:i32".to_string()),
        TokKind::Sym("b:i32".to_string()),
        TokKind::RBrack,
        TokKind::LParen,
        TokKind::Sym("+".to_string()),
        TokKind::Sym("a".to_string()),
        TokKind::Sym("b".to_string()),
        TokKind::RParen,
        TokKind::RParen,
    ];
    assert_eq!(kinds, expected);
}

#[test]
fn lexes_string_escapes() {
    let src = "(darcy.io/dbg \"a\\n\\t\\\"b\\\\\")";
    let toks = lex(src).expect("lex ok");
    let mut found = false;
    for t in toks {
        if let TokKind::Str(s) = t.kind {
            assert_eq!(s, "a\n\t\"b\\");
            found = true;
            break;
        }
    }
    assert!(found, "expected string token");
}

#[test]
fn lexes_braces() {
    let src = "{:a 1}";
    let toks = lex(src).expect("lex ok");
    let kinds: Vec<TokKind> = toks.into_iter().map(|t| t.kind).collect();
    let expected = vec![
        TokKind::LBrace,
        TokKind::Sym(":a".to_string()),
        TokKind::Int(1),
        TokKind::RBrace,
    ];
    assert_eq!(kinds, expected);
}

#[test]
fn lexes_sets() {
    let src = "#{:a 1}";
    let toks = lex(src).expect("lex ok");
    let kinds: Vec<TokKind> = toks.into_iter().map(|t| t.kind).collect();
    let expected = vec![
        TokKind::LSet,
        TokKind::Sym(":a".to_string()),
        TokKind::Int(1),
        TokKind::RBrace,
    ];
    assert_eq!(kinds, expected);
}

#[test]
fn lexes_namespaced_and_auto_symbols() {
    let src = "{:a 1 :foo.bar/a 2 ::a 3 ::m/a 4}";
    let toks = lex(src).expect("lex ok");
    let kinds: Vec<TokKind> = toks.into_iter().map(|t| t.kind).collect();
    let expected = vec![
        TokKind::LBrace,
        TokKind::Sym(":a".to_string()),
        TokKind::Int(1),
        TokKind::Sym(":foo.bar/a".to_string()),
        TokKind::Int(2),
        TokKind::Sym("::a".to_string()),
        TokKind::Int(3),
        TokKind::Sym("::m/a".to_string()),
        TokKind::Int(4),
        TokKind::RBrace,
    ];
    assert_eq!(kinds, expected);
}

#[test]
fn lexes_reader_macros() {
    let src = "`(list ~a ~@b) ^:meta x";
    let toks = lex(src).expect("lex ok");
    assert!(toks.iter().any(|t| matches!(t.kind, TokKind::SyntaxQuote)));
    assert!(toks.iter().any(|t| matches!(t.kind, TokKind::Unquote)));
    assert!(toks
        .iter()
        .any(|t| matches!(t.kind, TokKind::UnquoteSplicing)));
    assert!(toks.iter().any(|t| matches!(t.kind, TokKind::Meta)));
}
