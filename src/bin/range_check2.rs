use fibonacci::range_check::example2::{RangeCheckChip, RangeCheckConfig};
use halo2_proofs::{
    arithmetic::Field, circuit::SimpleFloorPlanner, dev::MockProver, halo2curves::pasta::Fp,
    plonk::Circuit,
};

#[derive(Default)]
struct RangeCheckCircuit<F, const RANGE: usize> {
    value: Option<F>,
}

impl<F: Field, const RANGE: usize> Circuit<F> for RangeCheckCircuit<F, RANGE> {
    type Config = RangeCheckConfig<F, RANGE>;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut halo2_proofs::plonk::ConstraintSystem<F>) -> Self::Config {
        RangeCheckChip::<F, RANGE>::configure(meta)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl halo2_proofs::circuit::Layouter<F>,
    ) -> Result<(), halo2_proofs::plonk::Error> {
        let range_check_chip = RangeCheckChip::<F, RANGE>::new(config);

        range_check_chip.load_table(layouter.namespace(|| "load"))?;

        range_check_chip.assign(layouter.namespace(|| "assign"), self.value)
    }
}

fn main() {
    let k = 14;
    let circuit = RangeCheckCircuit::<_, 1024> {
        value: Some(Fp::from(1023)),
    };

    let prover = MockProver::run(k, &circuit, vec![]).unwrap();
    prover.assert_satisfied();
}
