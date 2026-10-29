//! Prints the width of "weight|text" lines from stdin at font size 100:  cargo run -p gxoled_ui --example measure
use std::io::BufRead;
fn main() {
    let e = gxoled_ui::engine::Engine::new();
    for line in std::io::stdin().lock().lines().map_while(Result::ok) {
        let Some((w, t)) = line.split_once('|') else { continue };
        let weight: u16 = w.trim().parse().unwrap_or(400);
        println!("{:.3}\t{}", e.text_width(t, 100.0, weight, 0.0), t);
    }
}
