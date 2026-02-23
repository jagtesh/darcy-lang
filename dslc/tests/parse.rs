use dslc::{lex, parse_toplevel, Expr, Parser, Top, Ty};

#[test]
fn parses_struct_and_fn() {
    let src = "(defrecord order [qty:u32]) (defn total [o:order] o.qty)";
    let toks = lex(src).expect("lex ok");
    let mut parser = Parser::new(toks);
    let sexps = parser.parse_all().expect("parse sexps");
    let tops = parse_toplevel(&sexps).expect("parse toplevel");

    assert_eq!(tops.len(), 2);
    match &tops[0] {
        Top::Struct(sd) => assert_eq!(sd.name, "order"),
        _ => panic!("expected struct"),
    }
    match &tops[1] {
        Top::Func(fd) => assert_eq!(fd.name, "total"),
        _ => panic!("expected function"),
    }
}

#[test]
fn rejects_reserved_keyword_as_name() {
    let src = "(defn vec [x] x)";
    let toks = lex(src).expect("lex ok");
    let mut parser = Parser::new(toks);
    let sexps = parser.parse_all().expect("parse sexps");
    let err = parse_toplevel(&sexps).expect_err("expected error");
    assert!(err.message.contains("reserved keyword"), "{}", err.message);
}

#[test]
fn parses_map_literal_and_symbols() {
    let src = "(defn demo [] {:a 1 :b nil :c true :d false})";
    let toks = lex(src).expect("lex ok");
    let mut parser = Parser::new(toks);
    let sexps = parser.parse_all().expect("parse sexps");
    let tops = parse_toplevel(&sexps).expect("parse toplevel");

    match &tops[0] {
        Top::Func(fd) => match &fd.body {
            Expr::MapLit { entries, .. } => {
                assert_eq!(entries.len(), 4);
                assert!(matches!(entries[0].0, Expr::SymbolLit(_, _)));
                assert!(matches!(entries[1].1, Expr::Unit(_)));
                assert!(matches!(entries[2].1, Expr::Bool(true, _)));
                assert!(matches!(entries[3].1, Expr::Bool(false, _)));
            }
            _ => panic!("expected map literal"),
        },
        _ => panic!("expected function"),
    }
}

#[test]
fn parses_symbol_literal_forms() {
    let src = "(defn main [] {:a 1 :foo.bar/a 2 ::a 3 ::m/a 4})";
    let toks = lex(src).expect("lex ok");
    let mut parser = Parser::new(toks);
    let sexps = parser.parse_all().expect("parse sexps");
    let tops = parse_toplevel(&sexps).expect("parse toplevel");

    match &tops[0] {
        Top::Func(fd) => match &fd.body {
            Expr::MapLit { entries, .. } => {
                assert!(matches!(entries[0].0, Expr::SymbolLit(ref s, _) if s == ":a"));
                assert!(matches!(entries[1].0, Expr::SymbolLit(ref s, _) if s == ":foo.bar/a"));
                assert!(matches!(entries[2].0, Expr::SymbolLit(ref s, _) if s == "::a"));
                assert!(matches!(entries[3].0, Expr::SymbolLit(ref s, _) if s == "::m/a"));
            }
            _ => panic!("expected map literal"),
        },
        _ => panic!("expected function"),
    }
}

#[test]
fn rejects_expression_type_ascription_shorthand() {
    let src = "(defn main [] value:i64)";
    let toks = lex(src).expect("lex ok");
    let mut parser = Parser::new(toks);
    let sexps = parser.parse_all().expect("parse sexps");
    let err = parse_toplevel(&sexps).expect_err("expected error");
    assert!(err.message.contains("(type expr Type)"), "{}", err.message);
}

#[test]
fn rejects_malformed_symbol_literals() {
    let cases = [
        "(defn main [] :)",
        "(defn main [] ::)",
        "(defn main [] :/x)",
        "(defn main [] ::/x)",
    ];
    for src in cases {
        let toks = lex(src).expect("lex ok");
        let mut parser = Parser::new(toks);
        let sexps = parser.parse_all().expect("parse sexps");
        let err = parse_toplevel(&sexps).expect_err("expected error");
        assert!(
            err.message.contains("symbol"),
            "src={src} err={}",
            err.message
        );
    }
}

#[test]
fn parses_case_alias() {
    let src = "(defenum outcome (ok [v:i32]) (err [msg:string])) (defn demo [o:outcome] (case o (ok (v x) x) (_ 0)))";
    let toks = lex(src).expect("lex ok");
    let mut parser = Parser::new(toks);
    let sexps = parser.parse_all().expect("parse sexps");
    let tops = parse_toplevel(&sexps).expect("parse toplevel");

    match &tops[1] {
        Top::Func(fd) => assert!(matches!(fd.body, Expr::Match { .. })),
        _ => panic!("expected function"),
    }
}

#[test]
fn rejects_grouped_case_bindings_with_helpful_message() {
    let src = "(defenum shape (triangle [base height])) (defn shape-name [s] (case s (triangle (base b height h) \"triangle\")))";
    let toks = lex(src).expect("lex ok");
    let mut parser = Parser::new(toks);
    let sexps = parser.parse_all().expect("parse sexps");
    let err = parse_toplevel(&sexps).expect_err("expected error");
    assert!(
        err.message.contains("separate pairs"),
        "unexpected error: {}",
        err.message
    );
    assert!(
        err.message.contains("(base b) (height h)"),
        "unexpected error: {}",
        err.message
    );
}

#[test]
fn parses_cond_and_set_literals() {
    let src = "(defn demo [x:i32] (cond (true (hashset 1 2)) (else #{3 4})))";
    let toks = lex(src).expect("lex ok");
    let mut parser = Parser::new(toks);
    let sexps = parser.parse_all().expect("parse sexps");
    let tops = parse_toplevel(&sexps).expect("parse toplevel");

    match &tops[0] {
        Top::Func(fd) => match &fd.body {
            Expr::If { .. } => {}
            _ => panic!("expected cond to lower to if"),
        },
        _ => panic!("expected function"),
    }
}

#[test]
fn rejects_cond_else_not_last() {
    let src = "(defn demo [x:i32] (cond (else 1) (true 2)))";
    let toks = lex(src).expect("lex ok");
    let mut parser = Parser::new(toks);
    let sexps = parser.parse_all().expect("parse sexps");
    let err = parse_toplevel(&sexps).expect_err("expected error");
    assert!(err.message.contains("else"), "{}", err.message);
}

#[test]
fn parses_discard_reader() {
    let src = "(defn main [] (do 1 #_ 2 3))";
    let toks = lex(src).expect("lex ok");
    let mut parser = Parser::new(toks);
    let sexps = parser.parse_all().expect("parse sexps");
    let tops = parse_toplevel(&sexps).expect("parse toplevel");

    match &tops[0] {
        Top::Func(fd) => match &fd.body {
            Expr::Do { exprs, .. } => {
                assert_eq!(exprs.len(), 2);
                assert!(matches!(exprs[0], Expr::Int(1, _)));
                assert!(matches!(exprs[1], Expr::Int(3, _)));
            }
            _ => panic!("expected do"),
        },
        _ => panic!("expected function"),
    }
}

#[test]
fn parses_commas_as_whitespace() {
    let src = "(defn main [] (list 1, 2, 3))";
    let toks = lex(src).expect("lex ok");
    let mut parser = Parser::new(toks);
    let sexps = parser.parse_all().expect("parse sexps");
    let tops = parse_toplevel(&sexps).expect("parse toplevel");
    assert_eq!(tops.len(), 1);
}

#[test]
fn parses_predicate_function_name() {
    let src = "(defn empty? [xs] true) (defn main [] (empty? []))";
    let toks = lex(src).expect("lex ok");
    let mut parser = Parser::new(toks);
    let sexps = parser.parse_all().expect("parse sexps");
    let tops = parse_toplevel(&sexps).expect("parse toplevel");
    match &tops[0] {
        Top::Func(fd) => assert_eq!(fd.name, "empty?"),
        _ => panic!("expected function"),
    }
}

#[test]
fn parses_compiler_type_aliases() {
    let src = "(defrecord sample [n:int xs:vec<float> label:str]) (defn score [x:float y:uint] (+ x (cast y f64)))";
    let toks = lex(src).expect("lex ok");
    let mut parser = Parser::new(toks);
    let sexps = parser.parse_all().expect("parse sexps");
    let tops = parse_toplevel(&sexps).expect("parse toplevel");

    match &tops[0] {
        Top::Struct(sd) => {
            assert_eq!(sd.fields.len(), 3);
            assert!(matches!(sd.fields[0].ty, Ty::Named(ref n) if n == "i64"));
            assert!(matches!(
                sd.fields[1].ty,
                Ty::Vec(ref inner) if matches!(inner.as_ref(), Ty::Named(n) if n == "f64")
            ));
            assert!(matches!(sd.fields[2].ty, Ty::Named(ref n) if n == "string"));
        }
        _ => panic!("expected struct"),
    }
    match &tops[1] {
        Top::Func(fd) => {
            assert_eq!(fd.params.len(), 2);
            assert!(matches!(fd.params[0].ann, Some(Ty::Named(ref n)) if n == "f64"));
            assert!(matches!(fd.params[1].ann, Some(Ty::Named(ref n)) if n == "u64"));
        }
        _ => panic!("expected function"),
    }
}

#[test]
fn parses_struct_fields_with_optional_types() {
    let src = "(defrecord sample [a b:f64 c])";
    let toks = lex(src).expect("lex ok");
    let mut parser = Parser::new(toks);
    let sexps = parser.parse_all().expect("parse sexps");
    let tops = parse_toplevel(&sexps).expect("parse toplevel");

    match &tops[0] {
        Top::Struct(sd) => {
            assert_eq!(sd.fields.len(), 3);
            assert!(matches!(sd.fields[0].ty, Ty::Unknown));
            assert!(matches!(sd.fields[1].ty, Ty::Named(ref n) if n == "f64"));
            assert!(matches!(sd.fields[2].ty, Ty::Unknown));
        }
        _ => panic!("expected struct"),
    }
}

#[test]
fn parses_union_variant_fields_with_optional_types() {
    let src = "(defenum opt (some [x y:i64]))";
    let toks = lex(src).expect("lex ok");
    let mut parser = Parser::new(toks);
    let sexps = parser.parse_all().expect("parse sexps");
    let tops = parse_toplevel(&sexps).expect("parse toplevel");

    match &tops[0] {
        Top::Union(ud) => {
            assert_eq!(ud.variants.len(), 1);
            assert_eq!(ud.variants[0].fields.len(), 2);
            assert!(matches!(ud.variants[0].fields[0].ty, Ty::Unknown));
            assert!(matches!(ud.variants[0].fields[1].ty, Ty::Named(ref n) if n == "i64"));
        }
        _ => panic!("expected union"),
    }
}
