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

            let a = meta.query_advice(config.col, Rotation::prev());
            let b = meta.query_advice(config.col, Rotation::cur());
            let c = meta.query_advice(config.col, Rotation::next());

            vec![sel * (a + b - c)]
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
                self.config.sel.enable(&mut region, 1)?;

                let a = a.ok_or(plonk::Error::Synthesis)?;
                let b = b.ok_or(plonk::Error::Synthesis)?;
                let c = a + b;

                region.assign_advice(|| "a", self.config.col, 0, || Value::known(a))?;
                let cell_b =
                    region.assign_advice(|| "b", self.config.col, 1, || Value::known(b))?;
                let cell_c =
                    region.assign_advice(|| "c", self.config.col, 2, || Value::known(c))?;

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
                self.config.sel.enable(&mut region, 1)?;

                let c = prev_b.value().map(|v| *v) + prev_c.value();

                prev_b.copy_advice(|| "copy b", &mut region, self.config.col, 0)?;
                prev_c.copy_advice(|| "copy c", &mut region, self.config.col, 1)?;
                region.assign_advice(|| "assign c", self.config.col, 2, || c)
            },
        )
    }

    fn expose_public(
        &self,
        mut layouter: impl Layouter<F>,
        public: AssignedCell<F, F>,
    ) -> Result<(), plonk::Error> {
        layouter.constrain_instance(public.cell(), self.config.instance, 0)
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
        let fibo_config = FiboConfig::new(meta);
        FiboChip::configure(meta, &fibo_config);
        fibo_config
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), plonk::Error> {
        let fibo_chip = FiboChip::new(config);

        let (mut prev_b, mut prev_c) =
            fibo_chip.assign_init(layouter.namespace(|| "init"), &self.a, &self.b)?;

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
    let k = 6;
    let circuit = FiboCircuit {
        a: Some(Fp::from(0)),
        b: Some(Fp::from(1)),
    };

    let public = Fp::from(89);

    let prover = MockProver::run(k, &circuit, vec![vec![public]]).unwrap();
    prover.assert_satisfied();

    plot_layout("fib-2-layout.png", "Fibo2 Layout", k, &circuit);
}
