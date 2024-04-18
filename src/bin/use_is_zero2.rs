use fibonacci::is_zero2::{IsZero2Chip, IsZero2Config};
use halo2_proofs::{
    arithmetic::Field,
    circuit::{AssignedCell, Layouter, SimpleFloorPlanner, Value},
    dev::MockProver,
    halo2curves::pasta::Fp,
    plonk::{
        self, Advice, Circuit, Column, ConstraintSystem, Constraints, Expression, Instance,
        Selector,
    },
    poly::Rotation,
};

#[derive(Clone)]
struct FooConfig<F> {
    col: Column<Advice>,
    sel: Selector,
    instance: Column<Instance>,

    is_zero_config: IsZero2Config<F>,
}

struct FooChip<F> {
    config: FooConfig<F>,
}

impl<F: Field> FooChip<F> {
    fn new(config: FooConfig<F>) -> Self {
        Self { config }
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> FooConfig<F> {
        let col = meta.advice_column();
        let sel = meta.selector();
        let instance = meta.instance_column();

        meta.enable_equality(col);
        meta.enable_equality(instance);

        let is_zero_config = IsZero2Chip::configure(
            meta,
            |meta| meta.query_selector(sel),
            |meta| {
                meta.query_advice(col, Rotation::cur()) - meta.query_advice(col, Rotation::next())
            },
        );

        let config = FooConfig {
            col,
            sel,
            instance,
            is_zero_config,
        };

        meta.create_gate("foo", |meta| {
            let sel = meta.query_selector(sel);

            let a = meta.query_advice(config.col, Rotation::cur());
            let b = meta.query_advice(config.col, Rotation::next());
            let c = meta.query_advice(config.col, Rotation(2));
            let instance = meta.query_instance(config.instance, Rotation::cur());

            Constraints::with_selector(
                sel,
                [
                    (
                        "a == b",
                        config.is_zero_config.expr().clone() * (c - instance.clone()),
                    ),
                    (
                        "a != b",
                        (config.is_zero_config.expr().clone() - Expression::Constant(F::ONE))
                            * (a - b - instance),
                    ),
                ],
            )
        });

        config
    }

    fn assign(
        &self,
        mut layouter: impl Layouter<F>,
        a: &Option<F>,
        b: &Option<F>,
        c: &Option<F>,
    ) -> Result<AssignedCell<F, F>, plonk::Error> {
        layouter.assign_region(
            || "assign foo",
            |mut region| {
                self.config.sel.enable(&mut region, 0)?;

                let a = a.ok_or(plonk::Error::Synthesis)?;
                let b = b.ok_or(plonk::Error::Synthesis)?;
                let c = c.ok_or(plonk::Error::Synthesis)?;
                let output = if a == b { c } else { a - b };

                let is_zero_chip = IsZero2Chip::new(self.config.is_zero_config.clone());
                is_zero_chip.assign(&mut region, (a - b).invert().unwrap_or(F::ZERO))?;

                region.assign_advice(|| "a", self.config.col, 0, || Value::known(a))?;
                region.assign_advice(|| "b", self.config.col, 1, || Value::known(b))?;
                region.assign_advice(|| "output", self.config.col, 2, || Value::known(output))
            },
        )
    }

    fn expose_public(
        &self,
        mut layouter: impl Layouter<F>,
        cell: AssignedCell<F, F>,
    ) -> Result<(), plonk::Error> {
        layouter.constrain_instance(cell.cell(), self.config.instance, 0)
    }
}

#[derive(Default)]
struct FooCircuit<F> {
    a: Option<F>,
    b: Option<F>,
    c: Option<F>,
}

impl<F: Field> Circuit<F> for FooCircuit<F> {
    type Config = FooConfig<F>;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        FooChip::configure(meta)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), plonk::Error> {
        let foo_chip = FooChip::new(config);

        let cell = foo_chip.assign(layouter.namespace(|| "assign"), &self.a, &self.b, &self.c)?;

        foo_chip.expose_public(layouter.namespace(|| "expose public"), cell)
    }
}

fn main() {
    let k = 4;

    let a = Fp::from(11);
    let b = Fp::from(11);

    let circuit = FooCircuit {
        a: Some(a),
        b: Some(b),
        c: Some(Fp::from(222)),
    };

    let public_output = vec![Fp::from(222)];

    let prover = MockProver::run(k, &circuit, vec![public_output]).unwrap();
    prover.assert_satisfied();
}
