pub mod is_zero;
pub mod is_zero2;
pub mod range_check;

use halo2_proofs::{arithmetic::Field, plonk::Circuit};

pub fn plot_layout<P: AsRef<std::path::Path>, F: Field>(
    path: P,
    title: &str,
    k: u32,
    circuit: &impl Circuit<F>,
) {
    use plotters::prelude::*;
    let root = BitMapBackend::new(&path, (1024, 3096)).into_drawing_area();
    root.fill(&WHITE).unwrap();
    let root = root.titled(title, ("sans-serif", 60)).unwrap();

    halo2_proofs::dev::CircuitLayout::default()
        .render(k, circuit, &root)
        .unwrap();
}
