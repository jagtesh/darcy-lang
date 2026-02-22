use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::datum::{datum_span, Datum};
use crate::diag::{Diag, DslResult, Span};
use crate::lexer::lex;
use crate::reader::Reader;

#[derive(Debug, Clone)]
struct MacroDef {
    params: Vec<String>,
    body: Datum,
}

#[derive(Default)]
struct MacroEnv {
    defs: BTreeMap<String, MacroDef>,
}

const MACRO_PRELUDE: &str = r#"
(defmacro defextern [name params ret rust-name]
  `(extern ~rust-name (defn ~name ~params ~ret)))

(defmacro defextern-record [name rust-name fields]
  `(extern ~rust-name (defrecord ~name ~@fields)))

(defmacro defextern-enum [name rust-name variants]
  `(extern ~rust-name (defenum ~name ~@variants)))
"#;

pub fn expand_program(forms: &[Datum]) -> DslResult<Vec<Datum>> {
    let mut env = MacroEnv::default();
    load_prelude_macros(&mut env)?;
    let mut out = Vec::new();
    for form in forms {
        if let Some(def) = parse_defmacro(form)? {
            env.defs.insert(def.0, def.1);
            continue;
        }
        let expanded = expand_form(&env, form, 0)?;
        out.push(expanded);
    }
    Ok(out)
}

fn load_prelude_macros(env: &mut MacroEnv) -> DslResult<()> {
    let toks = lex(MACRO_PRELUDE)?;
    let mut reader = Reader::new(toks);
    let forms = reader.parse_all()?;
    for form in forms {
        if let Some(def) = parse_defmacro(&form)? {
            env.defs.insert(def.0, def.1);
        } else {
            return Err(Diag::new("invalid macro prelude form").with_span(datum_span(&form)));
        }
    }
    Ok(())
}

fn parse_defmacro(form: &Datum) -> DslResult<Option<(String, MacroDef)>> {
    let (items, span) = match form {
        Datum::List(items, sp) => (items, sp),
        _ => return Ok(None),
    };
    if items.is_empty() {
        return Ok(None);
    }
    let head = match &items[0] {
        Datum::Symbol(s, _) => s.as_str(),
        _ => return Ok(None),
    };
    if head != "defmacro" {
        return Ok(None);
    }
    if items.len() != 4 {
        return Err(
            Diag::new("defmacro form is (defmacro name [params] body)").with_span(span.clone())
        );
    }
    let name = match &items[1] {
        Datum::Symbol(s, _) => s.clone(),
        _ => {
            return Err(Diag::new("defmacro name must be a symbol").with_span(datum_span(&items[1])))
        }
    };
    let params = match &items[2] {
        Datum::Vec(p, _) => parse_param_names(p)?,
        _ => {
            return Err(
                Diag::new("defmacro params must be a vector").with_span(datum_span(&items[2]))
            )
        }
    };
    let body = items[3].clone();
    Ok(Some((name, MacroDef { params, body })))
}

fn parse_param_names(items: &[Datum]) -> DslResult<Vec<String>> {
    let mut out = Vec::new();
    for it in items {
        match it {
            Datum::Symbol(s, _) => out.push(s.clone()),
            _ => return Err(Diag::new("macro params must be symbols").with_span(datum_span(it))),
        }
    }
    Ok(out)
}

fn expand_form(env: &MacroEnv, form: &Datum, depth: usize) -> DslResult<Datum> {
    if depth > 256 {
        return Err(Diag::new("macro expansion limit exceeded").with_span(datum_span(form)));
    }
    if let Datum::List(items, sp) = form {
        if items.is_empty() {
            return Ok(form.clone());
        }
        if let Datum::Symbol(name, _) = &items[0] {
            if let Some(mac) = env.defs.get(name) {
                let args = items[1..].to_vec();
                let expanded = eval_macro(mac, &args)?;
                return expand_form(env, &expanded, depth + 1);
            }
        }
        let mut out = Vec::new();
        for it in items {
            out.push(expand_form(env, it, depth + 1)?);
        }
        return Ok(Datum::List(out, sp.clone()));
    }
    if let Datum::Meta { meta, form, span } = form {
        let meta = expand_form(env, meta, depth + 1)?;
        let form = expand_form(env, form, depth + 1)?;
        return Ok(Datum::Meta {
            meta: Box::new(meta),
            form: Box::new(form),
            span: span.clone(),
        });
    }
    if let Datum::Vec(items, sp) = form {
        let mut out = Vec::new();
        for it in items {
            out.push(expand_form(env, it, depth + 1)?);
        }
        return Ok(Datum::Vec(out, sp.clone()));
    }
    if let Datum::Map(entries, sp) = form {
        let mut out = Vec::new();
        for (k, v) in entries {
            out.push((
                expand_form(env, k, depth + 1)?,
                expand_form(env, v, depth + 1)?,
            ));
        }
        return Ok(Datum::Map(out, sp.clone()));
    }
    if let Datum::Set(items, sp) = form {
        let mut out = Vec::new();
        for it in items {
            out.push(expand_form(env, it, depth + 1)?);
        }
        return Ok(Datum::Set(out, sp.clone()));
    }
    Ok(form.clone())
}

fn eval_macro(def: &MacroDef, args: &[Datum]) -> DslResult<Datum> {
    let mut env = EvalEnv::new();
    bind_params(&mut env, &def.params, args, &datum_span(&def.body))?;
    eval(&def.body, &mut env)
}

#[derive(Default)]
struct EvalEnv {
    vars: BTreeMap<String, Datum>,
}

impl EvalEnv {
    fn new() -> Self {
        Self {
            vars: BTreeMap::new(),
        }
    }

    fn with_parent(parent: &EvalEnv) -> Self {
        Self {
            vars: parent.vars.clone(),
        }
    }

    fn get(&self, name: &str) -> Option<Datum> {
        self.vars.get(name).cloned()
    }

    fn set(&mut self, name: &str, val: Datum) {
        self.vars.insert(name.to_string(), val);
    }
}

fn bind_params(env: &mut EvalEnv, params: &[String], args: &[Datum], span: &Span) -> DslResult<()> {
    let mut idx = 0usize;
    let mut pi = 0usize;
    while pi < params.len() {
        if params[pi] == "&" {
            let name = params.get(pi + 1).ok_or_else(|| {
                Diag::new("macro rest params must be '& name'").with_span(span.clone())
            })?;
            let rest = args[idx..].to_vec();
            env.set(name, Datum::List(rest, span.clone()));
            return Ok(());
        }
        let arg = args
            .get(idx)
            .ok_or_else(|| Diag::new("macro arity mismatch").with_span(span.clone()))?;
        env.set(&params[pi], arg.clone());
        idx += 1;
        pi += 1;
    }
    if idx != args.len() {
        return Err(Diag::new("macro arity mismatch").with_span(span.clone()));
    }
    Ok(())
}

fn eval(form: &Datum, env: &mut EvalEnv) -> DslResult<Datum> {
    match form {
        Datum::Symbol(name, sp) => env.get(name).ok_or_else(|| {
            Diag::new(format!("unknown symbol '{}' in macro", name)).with_span(sp.clone())
        }),
        Datum::List(items, sp) => eval_list(items, sp, env),
        Datum::Meta { meta, form, span } => {
            let meta = eval(meta, env)?;
            let form = eval(form, env)?;
            Ok(Datum::Meta {
                meta: Box::new(meta),
                form: Box::new(form),
                span: span.clone(),
            })
        }
        Datum::Vec(items, sp) => {
            let mut out = Vec::new();
            for it in items {
                out.push(eval(it, env)?);
            }
            Ok(Datum::Vec(out, sp.clone()))
        }
        Datum::Map(entries, sp) => {
            let mut out = Vec::new();
            for (k, v) in entries {
                out.push((eval(k, env)?, eval(v, env)?));
            }
            Ok(Datum::Map(out, sp.clone()))
        }
        Datum::Set(items, sp) => {
            let mut out = Vec::new();
            for it in items {
                out.push(eval(it, env)?);
            }
            Ok(Datum::Set(out, sp.clone()))
        }
        Datum::Str(..)
        | Datum::Int(..)
        | Datum::Float(..)
        | Datum::Bool(..)
        | Datum::Keyword(..)
        | Datum::Nil(..) => Ok(form.clone()),
    }
}

fn eval_list(items: &[Datum], sp: &Span, env: &mut EvalEnv) -> DslResult<Datum> {
    if items.is_empty() {
        return Ok(Datum::List(Vec::new(), sp.clone()));
    }
    let head = match &items[0] {
        Datum::Symbol(s, _) => s.as_str(),
        _ => {
            return Err(
                Diag::new("macro call head must be a symbol").with_span(datum_span(&items[0]))
            )
        }
    };
    match head {
        "quote" => {
            if items.len() != 2 {
                return Err(Diag::new("quote form is (quote x)").with_span(sp.clone()));
            }
            return Ok(items[1].clone());
        }
        "syntax-quote" => {
            if items.len() != 2 {
                return Err(
                    Diag::new("syntax-quote form is (syntax-quote x)").with_span(sp.clone())
                );
            }
            let mut state = GensymState::new();
            return syntax_quote(&items[1], env, &mut state);
        }
        "if" => {
            if items.len() < 3 || items.len() > 4 {
                return Err(Diag::new("if form is (if cond then [else])").with_span(sp.clone()));
            }
            let cond = eval(&items[1], env)?;
            if is_truthy(&cond) {
                return eval(&items[2], env);
            }
            if items.len() == 4 {
                return eval(&items[3], env);
            }
            return Ok(Datum::Nil(sp.clone()));
        }
        "do" => {
            let mut last = Datum::Nil(sp.clone());
            for it in items.iter().skip(1) {
                last = eval(it, env)?;
            }
            return Ok(last);
        }
        "let" => {
            if items.len() != 3 {
                return Err(Diag::new("let form is (let [bindings] body)").with_span(sp.clone()));
            }
            let bindings = match &items[1] {
                Datum::Vec(v, _) => v,
                _ => {
                    return Err(
                        Diag::new("let bindings must be a vector").with_span(datum_span(&items[1]))
                    )
                }
            };
            if bindings.len() % 2 != 0 {
                return Err(
                    Diag::new("let bindings must be name/value pairs").with_span(sp.clone())
                );
            }
            let mut local = EvalEnv::with_parent(env);
            let mut idx = 0usize;
            while idx < bindings.len() {
                let name = match &bindings[idx] {
                    Datum::Symbol(s, _) => s.clone(),
                    _ => {
                        return Err(Diag::new("let binding must be a symbol")
                            .with_span(datum_span(&bindings[idx])))
                    }
                };
                let val = eval(&bindings[idx + 1], &mut local)?;
                local.set(&name, val);
                idx += 2;
            }
            return eval(&items[2], &mut local);
        }
        "list" => {
            let mut out = Vec::new();
            for it in items.iter().skip(1) {
                out.push(eval(it, env)?);
            }
            return Ok(Datum::List(out, sp.clone()));
        }
        "list*" => {
            if items.len() < 2 {
                return Err(Diag::new("list* expects at least 1 argument").with_span(sp.clone()));
            }
            let mut out = Vec::new();
            for it in items.iter().skip(1).take(items.len() - 2) {
                out.push(eval(it, env)?);
            }
            let tail = eval(items.last().expect("list* tail"), env)?;
            match tail {
                Datum::List(items, _) | Datum::Vec(items, _) => {
                    out.extend(items);
                    return Ok(Datum::List(out, sp.clone()));
                }
                _ => {
                    return Err(Diag::new("list* tail must be list or vector").with_span(sp.clone()))
                }
            }
        }
        "vec" => {
            let mut out = Vec::new();
            for it in items.iter().skip(1) {
                out.push(eval(it, env)?);
            }
            return Ok(Datum::Vec(out, sp.clone()));
        }
        "hash-map" => {
            if (items.len() - 1) % 2 != 0 {
                return Err(
                    Diag::new("hash-map expects even number of forms").with_span(sp.clone())
                );
            }
            let mut entries = Vec::new();
            let mut idx = 1usize;
            while idx + 1 < items.len() {
                let k = eval(&items[idx], env)?;
                let v = eval(&items[idx + 1], env)?;
                entries.push((k, v));
                idx += 2;
            }
            return Ok(Datum::Map(entries, sp.clone()));
        }
        "hash-set" => {
            let mut out = Vec::new();
            for it in items.iter().skip(1) {
                out.push(eval(it, env)?);
            }
            return Ok(Datum::Set(out, sp.clone()));
        }
        "apply" => {
            if items.len() < 3 {
                return Err(Diag::new("apply expects function and args").with_span(sp.clone()));
            }
            let func = match &items[1] {
                Datum::Symbol(s, _) => s.clone(),
                _ => {
                    return Err(Diag::new("apply expects a symbol function")
                        .with_span(datum_span(&items[1])))
                }
            };
            let mut args = Vec::new();
            for it in items.iter().skip(2).take(items.len() - 3) {
                args.push(eval(it, env)?);
            }
            let tail = eval(items.last().expect("apply tail"), env)?;
            match tail {
                Datum::List(items, _) | Datum::Vec(items, _) => {
                    args.extend(items);
                }
                _ => {
                    return Err(Diag::new("apply tail must be list or vector").with_span(sp.clone()))
                }
            }
            let mut call = Vec::new();
            call.push(Datum::Symbol(func, sp.clone()));
            call.extend(args);
            return eval_list(&call, sp, env);
        }
        "map" => {
            if items.len() != 3 {
                return Err(Diag::new("map expects function and collection").with_span(sp.clone()));
            }
            let func = match &items[1] {
                Datum::Symbol(s, _) => s.clone(),
                _ => {
                    return Err(
                        Diag::new("map expects a symbol function").with_span(datum_span(&items[1]))
                    )
                }
            };
            let coll = eval(&items[2], env)?;
            let elems = match coll {
                Datum::List(items, _) | Datum::Vec(items, _) => items,
                _ => {
                    return Err(
                        Diag::new("map expects list or vector").with_span(datum_span(&items[2]))
                    )
                }
            };
            let mut out = Vec::new();
            for it in elems {
                let call = Datum::List(
                    vec![Datum::Symbol(func.clone(), sp.clone()), it],
                    sp.clone(),
                );
                out.push(eval(&call, env)?);
            }
            return Ok(Datum::List(out, sp.clone()));
        }
        "reduce" => {
            if items.len() < 3 || items.len() > 4 {
                return Err(
                    Diag::new("reduce expects (reduce f coll) or (reduce f init coll)")
                        .with_span(sp.clone()),
                );
            }
            let func = match &items[1] {
                Datum::Symbol(s, _) => s.clone(),
                _ => {
                    return Err(Diag::new("reduce expects a symbol function")
                        .with_span(datum_span(&items[1])))
                }
            };
            let (init, coll_form) = if items.len() == 4 {
                (Some(eval(&items[2], env)?), &items[3])
            } else {
                (None, &items[2])
            };
            let coll = eval(coll_form, env)?;
            let elems = match coll {
                Datum::List(items, _) | Datum::Vec(items, _) => items,
                _ => {
                    return Err(
                        Diag::new("reduce expects list or vector").with_span(datum_span(coll_form))
                    )
                }
            };
            let mut iter = elems.into_iter();
            let mut acc = if let Some(init) = init {
                init
            } else {
                iter.next()
                    .ok_or_else(|| Diag::new("reduce on empty collection").with_span(sp.clone()))?
            };
            for it in iter {
                let call = Datum::List(
                    vec![Datum::Symbol(func.clone(), sp.clone()), acc, it],
                    sp.clone(),
                );
                acc = eval(&call, env)?;
            }
            return Ok(acc);
        }
        "gensym" => {
            if items.len() > 2 {
                return Err(Diag::new("gensym expects 0 or 1 argument").with_span(sp.clone()));
            }
            let prefix = if items.len() == 2 {
                match eval(&items[1], env)? {
                    Datum::Str(s, _) => s,
                    Datum::Symbol(s, _) => s,
                    _ => {
                        return Err(Diag::new("gensym prefix must be a symbol or string")
                            .with_span(datum_span(&items[1])))
                    }
                }
            } else {
                "g".to_string()
            };
            let mut state = GensymState::new();
            return Ok(Datum::Symbol(state.gensym(&prefix), sp.clone()));
        }
        "cons" => {
            if items.len() != 3 {
                return Err(Diag::new("cons expects 2 arguments").with_span(sp.clone()));
            }
            let head_val = eval(&items[1], env)?;
            let tail = eval(&items[2], env)?;
            let mut out = Vec::new();
            out.push(head_val);
            match tail {
                Datum::List(items, _) => out.extend(items),
                Datum::Vec(items, _) => out.extend(items),
                _ => {
                    return Err(
                        Diag::new("cons expects list or vector").with_span(datum_span(&items[2]))
                    )
                }
            }
            return Ok(Datum::List(out, sp.clone()));
        }
        "concat" => {
            let mut out = Vec::new();
            for it in items.iter().skip(1) {
                let val = eval(it, env)?;
                match val {
                    Datum::List(items, _) => out.extend(items),
                    Datum::Vec(items, _) => out.extend(items),
                    _ => {
                        return Err(
                            Diag::new("concat expects lists or vectors").with_span(datum_span(it))
                        )
                    }
                }
            }
            return Ok(Datum::List(out, sp.clone()));
        }
        "first" => {
            if items.len() != 2 {
                return Err(Diag::new("first expects 1 argument").with_span(sp.clone()));
            }
            let val = eval(&items[1], env)?;
            match val {
                Datum::List(items, _) | Datum::Vec(items, _) => {
                    return Ok(items.into_iter().next().unwrap_or(Datum::Nil(sp.clone())));
                }
                _ => {
                    return Err(
                        Diag::new("first expects list or vector").with_span(datum_span(&items[1]))
                    )
                }
            }
        }
        "rest" => {
            if items.len() != 2 {
                return Err(Diag::new("rest expects 1 argument").with_span(sp.clone()));
            }
            let val = eval(&items[1], env)?;
            match val {
                Datum::List(items, _) | Datum::Vec(items, _) => {
                    let rest = items.into_iter().skip(1).collect();
                    return Ok(Datum::List(rest, sp.clone()));
                }
                _ => {
                    return Err(
                        Diag::new("rest expects list or vector").with_span(datum_span(&items[1]))
                    )
                }
            }
        }
        "nth" => {
            if items.len() != 3 {
                return Err(Diag::new("nth expects 2 arguments").with_span(sp.clone()));
            }
            let val = eval(&items[1], env)?;
            let idx = eval(&items[2], env)?;
            let idx = match idx {
                Datum::Int(v, _) => v as usize,
                _ => {
                    return Err(
                        Diag::new("nth expects integer index").with_span(datum_span(&items[2]))
                    )
                }
            };
            match val {
                Datum::List(items, _) | Datum::Vec(items, _) => {
                    return Ok(items.get(idx).cloned().unwrap_or(Datum::Nil(sp.clone())));
                }
                _ => {
                    return Err(
                        Diag::new("nth expects list or vector").with_span(datum_span(&items[1]))
                    )
                }
            }
        }
        "count" => {
            if items.len() != 2 {
                return Err(Diag::new("count expects 1 argument").with_span(sp.clone()));
            }
            let val = eval(&items[1], env)?;
            let len = match val {
                Datum::List(items, _) | Datum::Vec(items, _) => items.len(),
                Datum::Map(items, _) => items.len(),
                Datum::Set(items, _) => items.len(),
                _ => {
                    return Err(Diag::new("count expects list, vector, map, or set")
                        .with_span(datum_span(&items[1])))
                }
            };
            return Ok(Datum::Int(len as i64, sp.clone()));
        }
        "symbol" => {
            if items.len() != 2 {
                return Err(Diag::new("symbol expects 1 argument").with_span(sp.clone()));
            }
            let val = eval(&items[1], env)?;
            let name = match val {
                Datum::Str(s, _) => s,
                Datum::Symbol(s, _) => s,
                _ => {
                    return Err(Diag::new("symbol expects string or symbol")
                        .with_span(datum_span(&items[1])))
                }
            };
            return Ok(Datum::Symbol(name, sp.clone()));
        }
        "keyword" => {
            if items.len() != 2 {
                return Err(Diag::new("keyword expects 1 argument").with_span(sp.clone()));
            }
            let val = eval(&items[1], env)?;
            let name = match val {
                Datum::Str(s, _) => s,
                Datum::Symbol(s, _) => s,
                Datum::Keyword(s, _) => s,
                _ => {
                    return Err(Diag::new("keyword expects string or symbol")
                        .with_span(datum_span(&items[1])))
                }
            };
            return Ok(Datum::Keyword(name, sp.clone()));
        }
        _ => {}
    }
    Err(Diag::new(format!("unknown macro function '{}'", head)).with_span(sp.clone()))
}

fn is_truthy(v: &Datum) -> bool {
    !matches!(v, Datum::Bool(false, _) | Datum::Nil(_))
}

struct GensymState {
    map: BTreeMap<String, String>,
}

impl GensymState {
    fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }

    fn gensym(&mut self, base: &str) -> String {
        let id = next_gensym_id();
        format!("{}-g{}", base, id)
    }

    fn auto(&mut self, base: &str) -> String {
        if let Some(existing) = self.map.get(base) {
            return existing.clone();
        }
        let name = self.gensym(base);
        self.map.insert(base.to_string(), name.clone());
        name
    }
}

fn syntax_quote(form: &Datum, env: &mut EvalEnv, state: &mut GensymState) -> DslResult<Datum> {
    match form {
        Datum::List(items, sp) => {
            if let Some(Datum::Symbol(head, _)) = items.first() {
                if head == "unquote" {
                    if items.len() != 2 {
                        return Err(Diag::new("unquote form is (unquote x)").with_span(sp.clone()));
                    }
                    return eval(&items[1], env);
                }
                if head == "unquote-splicing" {
                    return Err(Diag::new("unquote-splicing not in list").with_span(sp.clone()));
                }
            }
            let mut out = Vec::new();
            for it in items {
                if let Datum::List(inner, _) = it {
                    if let Some(Datum::Symbol(head, _)) = inner.first() {
                        if head == "unquote-splicing" {
                            if inner.len() != 2 {
                                return Err(Diag::new(
                                    "unquote-splicing form is (unquote-splicing x)",
                                )
                                .with_span(datum_span(it)));
                            }
                            let val = eval(&inner[1], env)?;
                            match val {
                                Datum::List(items, _) | Datum::Vec(items, _) => {
                                    out.extend(items);
                                }
                                _ => {
                                    return Err(Diag::new(
                                        "unquote-splicing expects list or vector",
                                    )
                                    .with_span(datum_span(&inner[1])))
                                }
                            }
                            continue;
                        }
                    }
                }
                out.push(syntax_quote(it, env, state)?);
            }
            Ok(Datum::List(out, sp.clone()))
        }
        Datum::Vec(items, sp) => {
            let mut out = Vec::new();
            for it in items {
                if let Datum::List(inner, _) = it {
                    if let Some(Datum::Symbol(head, _)) = inner.first() {
                        if head == "unquote-splicing" {
                            if inner.len() != 2 {
                                return Err(Diag::new(
                                    "unquote-splicing form is (unquote-splicing x)",
                                )
                                .with_span(datum_span(it)));
                            }
                            let val = eval(&inner[1], env)?;
                            match val {
                                Datum::List(items, _) | Datum::Vec(items, _) => {
                                    out.extend(items);
                                }
                                _ => {
                                    return Err(Diag::new(
                                        "unquote-splicing expects list or vector",
                                    )
                                    .with_span(datum_span(&inner[1])))
                                }
                            }
                            continue;
                        }
                    }
                }
                out.push(syntax_quote(it, env, state)?);
            }
            Ok(Datum::Vec(out, sp.clone()))
        }
        Datum::Map(entries, sp) => {
            let mut out = Vec::new();
            for (k, v) in entries {
                out.push((syntax_quote(k, env, state)?, syntax_quote(v, env, state)?));
            }
            Ok(Datum::Map(out, sp.clone()))
        }
        Datum::Set(items, sp) => {
            let mut out = Vec::new();
            for it in items {
                out.push(syntax_quote(it, env, state)?);
            }
            Ok(Datum::Set(out, sp.clone()))
        }
        Datum::Meta { meta, form, span } => Ok(Datum::Meta {
            meta: Box::new(syntax_quote(meta, env, state)?),
            form: Box::new(syntax_quote(form, env, state)?),
            span: span.clone(),
        }),
        Datum::Symbol(name, sp) => {
            if let Some(base) = name.strip_suffix('#') {
                if base.is_empty() {
                    return Err(
                        Diag::new("auto-gensym requires a symbol prefix").with_span(sp.clone())
                    );
                }
                return Ok(Datum::Symbol(state.auto(base), sp.clone()));
            }
            Ok(Datum::Symbol(name.clone(), sp.clone()))
        }
        Datum::Keyword(..)
        | Datum::Str(..)
        | Datum::Int(..)
        | Datum::Float(..)
        | Datum::Bool(..)
        | Datum::Nil(..) => Ok(form.clone()),
    }
}

static GENSYM_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn next_gensym_id() -> usize {
    GENSYM_COUNTER.fetch_add(1, Ordering::SeqCst)
}
