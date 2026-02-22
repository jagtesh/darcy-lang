mod darcy_gen {
    include!(concat!(env!("OUT_DIR"), "/darcy_gen.rs"));
}

pub fn round_tick(price: f64) -> f64 {
    let tick = 0.01;
    (price / tick).round() * tick
}

fn make_bars() -> Vec<darcy_gen::Bar> {
    darcy_gen::gen_bars(10_000, 1337, 100.0)
}

fn main() {
    let bars = make_bars();
    let sig = darcy_gen::generate_signal(bars, 20, 50);
    println!("signal: {:?}", sig);
}
