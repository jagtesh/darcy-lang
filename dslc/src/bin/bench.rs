use std::env;
use std::time::Instant;

fn main() {
    let mut iters = 100_000usize;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--iters" {
            if let Some(v) = args.next() {
                iters = v.parse().unwrap_or(iters);
            }
        }
    }

    let data = sample_series(512);
    let threshold = 0.02;

    let start = Instant::now();
    let mut acc = 0.0;
    for _ in 0..iters {
        acc += dsl_equiv(&data, threshold);
    }
    let dur = start.elapsed();
    println!("dsl_equiv: {:?} (acc={})", dur, acc);

    #[cfg(feature = "cel")]
    {
        let start = Instant::now();
        let mut acc = 0.0;
        for _ in 0..iters {
            acc += cel_equiv(&data, threshold);
        }
        let dur = start.elapsed();
        println!("cel_equiv: {:?} (acc={})", dur, acc);
    }

    #[cfg(not(feature = "cel"))]
    {
        eprintln!("cel feature is disabled; build with --features cel to compare.");
    }
}

fn sample_series(n: usize) -> Vec<f64> {
    let mut out = Vec::with_capacity(n);
    let mut v = 1.0;
    for i in 0..n {
        v += (i as f64).sin() * 0.001;
        out.push(v);
    }
    out
}

// Equivalent of a simple DSL strategy:
// compute a 10-period SMA and return last value above a threshold.
fn dsl_equiv(values: &[f64], threshold: f64) -> f64 {
    let sma = simple_sma(values, 10);
    let last = *sma.last().unwrap_or(&0.0);
    if last > threshold { last } else { 0.0 }
}

fn simple_sma(values: &[f64], period: usize) -> Vec<f64> {
    if values.is_empty() || period == 0 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(values.len());
    let mut sum = 0.0;
    for i in 0..values.len() {
        sum += values[i];
        if i >= period {
            sum -= values[i - period];
        }
        let denom = if i + 1 < period { (i + 1) as f64 } else { period as f64 };
        out.push(sum / denom);
    }
    out
}

#[cfg(feature = "cel")]
fn cel_equiv(values: &[f64], threshold: f64) -> f64 {
    use cel::{Context, Program, Value};
    use std::sync::OnceLock;

    static PROGRAM: OnceLock<Program> = OnceLock::new();
    static CONTEXT: OnceLock<Context<'static>> = OnceLock::new();

    let program = PROGRAM.get_or_init(|| {
        let src = "last_sma(values, 10) > threshold ? last_sma(values, 10) : 0.0";
        Program::compile(src).expect("cel parse")
    });

    let ctx = CONTEXT.get_or_init(|| {
        let mut ctx = Context::default();
        ctx.add_variable("values", values.to_vec()).expect("cel values");
        ctx.add_variable("threshold", threshold).expect("cel threshold");
        ctx.add_function("last_sma", |vals: std::sync::Arc<Vec<Value>>, period: i64| -> f64 {
            let mut out = Vec::with_capacity(vals.len());
            for v in vals.iter() {
                match v {
                    Value::Float(f) => out.push(*f),
                    Value::Int(i) => out.push(*i as f64),
                    _ => return 0.0,
                }
            }
            let sma = simple_sma(&out, period as usize);
            *sma.last().unwrap_or(&0.0)
        });
        ctx
    });

    let value = program.execute(ctx).expect("cel exec");
    match value {
        Value::Float(v) => v,
        Value::Int(v) => v as f64,
        _ => 0.0,
    }
}
