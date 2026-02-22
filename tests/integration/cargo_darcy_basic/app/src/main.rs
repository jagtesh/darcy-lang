fn main() {
    let from_darcy_main = darcy_lib::main();
    let from_darcy_calc = darcy_lib::calc(4);
    let from_rust = (4 + 2 + 3) * 10;

    println!("darcy_main={} darcy_calc={} rust={}", from_darcy_main, from_darcy_calc, from_rust);
}
