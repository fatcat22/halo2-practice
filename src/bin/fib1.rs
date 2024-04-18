use std::marker::PhantomData;

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
    col_a: Column<Advice>,
    col_b: Column<Advice>,
    col_c: Column<Advice>,
    sel: Selector,
    instance: Column<Instance>,
}

impl FiboConfig {
    fn new<F: Field>(meta: &mut ConstraintSystem<F>) -> Self {
        let col_a = meta.advice_column();
        let col_b = meta.advice_column();
        let col_c = meta.advice_column();
        let sel = meta.selector();
        let instance = meta.instance_column();

        meta.enable_equality(col_a);
        meta.enable_equality(col_b);
        meta.enable_equality(col_c);
        meta.enable_equality(instance);

        Self {
            col_a,
            col_b,
            col_c,
            sel,
            instance,
        }
    }
}

struct FiboChip<F> {
    config: FiboConfig,
    _marker: PhantomData<F>,
}

impl<F: Field> FiboChip<F> {
    fn new(config: FiboConfig) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }

    fn configure(meta: &mut ConstraintSystem<F>, config: &FiboConfig) {
        meta.create_gate("fibonacci", |meta| {
            let sel = meta.query_selector(config.sel);

            let col_a = meta.query_advice(config.col_a, Rotation::cur());
            let col_b = meta.query_advice(config.col_b, Rotation::cur());
            let col_c = meta.query_advice(config.col_c, Rotation::cur());

            vec![sel * (col_a + col_b - col_c)]
        })
    }

    fn assign_init(
        &self,
        mut layouter: impl Layouter<F>,
        a: &Option<F>,
        b: &Option<F>,
    ) -> Result<(AssignedCell<F, F>, AssignedCell<F, F>), plonk::Error> {
        layouter.assign_region(
            || "assign init",
            |mut region| {
                self.config.sel.enable(&mut region, 0)?;

                let a = a.ok_or(plonk::Error::Synthesis)?;
                let b = b.ok_or(plonk::Error::Synthesis)?;
                let c = a + b;

                let _cell_a =
                    region.assign_advice(|| "init a", self.config.col_a, 0, || Value::known(a))?;
                let cell_b =
                    region.assign_advice(|| "init b", self.config.col_b, 0, || Value::known(b))?;
                let cell_c =
                    region.assign_advice(|| "init c", self.config.col_c, 0, || Value::known(c))?;
                Ok((cell_b, cell_c))
            },
        )
    }

    fn assign(
        &self,
        mut layouter: impl Layouter<F>,
        prev_b: &AssignedCell<F, F>,
        prev_c: &AssignedCell<F, F>,
    ) -> Result<AssignedCell<F, F>, plonk::Error> {
        layouter.assign_region(
            || "assign",
            |mut region| {
                self.config.sel.enable(&mut region, 0)?;

                let c = prev_b.value().and_then(|a| prev_c.value().map(|b| *a + *b));

                prev_b.copy_advice(|| "copy prev b", &mut region, self.config.col_a, 0)?;
                prev_c.copy_advice(|| "copy prev c", &mut region, self.config.col_b, 0)?;
                let cell_c = region.assign_advice(|| "assign c", self.config.col_c, 0, || c)?;
                Ok(cell_c)
            },
        )
    }

    fn expose_public(
        &self,
        mut layouter: impl Layouter<F>,
        output: AssignedCell<F, F>,
    ) -> Result<(), plonk::Error> {
        layouter.constrain_instance(output.cell(), self.config.instance, 0)
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
        mut layouter: impl halo2_proofs::circuit::Layouter<F>,
    ) -> Result<(), halo2_proofs::plonk::Error> {
        let fibo_chip = FiboChip::new(config);

        let (mut prev_b, mut prev_c) =
            fibo_chip.assign_init(layouter.namespace(|| "assign init"), &self.a, &self.b)?;

        for i in 1..10 {
            let cell_c = fibo_chip.assign(
                layouter.namespace(|| format!("assign-{}", i)),
                &prev_b,
                &prev_c,
            )?;

            prev_b = prev_c;
            prev_c = cell_c;
        }

        fibo_chip.expose_public(layouter.namespace(|| "expose public"), prev_c)
    }
}

fn main() {
    let k = 4;

    let circuit = FiboCircuit {
        a: Some(Fp::from(0)),
        b: Some(Fp::from(1)),
    };
    let output = Fp::from(89);

    let prover = MockProver::run(4, &circuit, vec![vec![output]]).unwrap();
    prover.assert_satisfied();

    plot_layout("fib1.png", "fib1 layout", k, &circuit);
}
