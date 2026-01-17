use dslc::{lex, parse_toplevel, Parser, Top};

#[test]
fn parses_struct_and_fn() {
    let src = "(defstruct order (qty u32)) (defn total [o:order] o.qty)";
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
