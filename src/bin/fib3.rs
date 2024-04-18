use fibonacci::plot_layout;
use halo2_proofs::{
    arithmetic::Field,
    circuit::{AssignedCell, Layouter, SimpleFloorPlanner, Value},
    dev::MockProver,
    halo2curves::pasta::Fp,
    plonk::{self, Advice, Circuit, Column, ConstraintSystem, Instance, Selector},
    poly::Rotation,
};

#[derive(Clone)]
struct FiboConfig {
    col: Column<Advice>,
    sel: Selector,
    instance: Column<Instance>,
}

impl FiboConfig {
    fn new<F: Field>(meta: &mut ConstraintSystem<F>) -> Self {
        let col = meta.advice_column();
        let sel = meta.selector();
        let instance = meta.instance_column();

        meta.enable_equality(col);
        meta.enable_equality(instance);

        Self { col, sel, instance }
    }
}

struct FiboChip {
    config: FiboConfig,
}

impl FiboChip {
    fn new(config: FiboConfig) -> Self {
        Self { config }
    }

    fn configure<F: Field>(meta: &mut ConstraintSystem<F>, config: &FiboConfig) {
        meta.create_gate("fibonacci", |meta| {
            let sel = meta.query_selector(config.sel);
            let a = meta.query_advice(config.col, Rotation::cur());
            let b = meta.query_advice(config.col, Rotation::next());
            let c = meta.query_advice(config.col, Rotation(2));

            vec![sel * (a + b - c)]
        })
    }

    fn assign<F: Field>(
        &self,
        mut layouter: impl Layouter<F>,
        a: &Option<F>,
        b: &Option<F>,
        nrows: usize,
    ) -> Result<AssignedCell<F, F>, plonk::Error> {
        layouter.assign_region(
            || "entire table",
            |mut region| {
                let a = a.ok_or(plonk::Error::Synthesis)?;
                let b = b.ok_or(plonk::Error::Synthesis)?;

                self.config.sel.enable(&mut region, 0)?;
                region.assign_advice(|| "init a", self.config.col, 0, || Value::known(a))?;
                let mut prev_b =
                    region.assign_advice(|| "init b", self.config.col, 1, || Value::known(b))?;
                let mut prev_c = region.assign_advice(
                    || "init c",
                    self.config.col,
                    2,
                    || Value::known(a + b),
                )?;

                for i in 3..nrows {
                    self.config.sel.enable(&mut region, i - 2)?;
                    let v = prev_b.value().cloned() + prev_c.value();
                    let cell_c =
                        region.assign_advice(|| format!("row-{}", i), self.config.col, i, || v)?;

                    prev_b = prev_c;
                    prev_c = cell_c;
                }

                Ok(prev_c)
            },
        )
    }

    fn expose_public<F: Field>(
        &self,
        mut layouter: impl Layouter<F>,
        final_cell: AssignedCell<F, F>,
    ) -> Result<(), plonk::Error> {
        layouter.constrain_instance(final_cell.cell(), self.config.instance, 0)
    }
}

#[derive(Default)]
struct FiboCircuit<F> {
    a: Option<F>,
    b: Option<F>,
}

impl<F: Field> Circuit<F> for FiboCircuit<F> {
    type Config = FiboConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let config = FiboConfig::new(meta);
        FiboChip::configure(meta, &config);
        config
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), plonk::Error> {
        let fibo_chip = FiboChip::new(config);

        let final_cell = fibo_chip.assign(layouter.namespace(|| "assign"), &self.a, &self.b, 10)?;

        fibo_chip.expose_public(layouter.namespace(|| "expose public"), final_cell)
    }
}

fn main() {
    let k = 4;

    let circuit = FiboCircuit {
        a: Some(Fp::from(0)),
        b: Some(Fp::from(1)),
    };

    let prover = MockProver::run(k, &circuit, vec![vec![Fp::from(34)]]).unwrap();
    prover.assert_satisfied();

    plot_layout("fib3.png", "fib3 layout", k, &circuit);
}
