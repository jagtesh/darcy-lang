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
