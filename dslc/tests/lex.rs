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
    let src = "(dbg \"a\\n\\t\\\"b\\\\\")";
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
