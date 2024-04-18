use fibonacci::is_zero::{IsZeroChip, IsZeroConfig};
use halo2_proofs::{
    arithmetic::Field,
    circuit::{Layouter, SimpleFloorPlanner, Value},
    dev::MockProver,
    halo2curves::pasta::Fp,
    plonk::{self, Advice, Circuit, Column, ConstraintSystem, Expression, Selector},
    poly::Rotation,
};

#[derive(Clone)]
struct FooConfig<F> {
    a: Column<Advice>,
    b: Column<Advice>,
    c: Column<Advice>,
    output: Column<Advice>,
    sel: Selector,
    is_zero_config: IsZeroConfig<F>,
}

struct FooChip<F> {
    config: FooConfig<F>,
}

impl<F: Field> FooChip<F> {
    fn new(config: FooConfig<F>) -> Self {
        Self { config }
    }

    fn configure(meta: &mut ConstraintSystem<F>, config: &FooConfig<F>) {
        meta.create_gate("if a == b {c} else {a - b}", |vcells| {
            let a = vcells.query_advice(config.a, Rotation::cur());
            let b = vcells.query_advice(config.b, Rotation::cur());
            let c = vcells.query_advice(config.c, Rotation::cur());
            let output = vcells.query_advice(config.output, Rotation::cur());
            let sel = vcells.query_selector(config.sel);

            vec![
                sel.clone() * config.is_zero_config.expr() * (c - output.clone()),
                sel * (Expression::Constant(F::ONE) - config.is_zero_config.expr())
                    * (a - b - output),
            ]
        });
    }

    fn assign(
        &self,
        mut layouter: impl Layouter<F>,
        a: &Option<F>,
        b: &Option<F>,
        c: &Option<F>,
    ) -> Result<(), plonk::Error> {
        let is_zero_chip = IsZeroChip::new(self.config.is_zero_config.clone());

        layouter.assign_region(
            || "foo",
            |mut region| {
                let a = a.ok_or(plonk::Error::Synthesis)?;
                let b = b.ok_or(plonk::Error::Synthesis)?;
                let c = c.ok_or(plonk::Error::Synthesis)?;
                let output = if a == b { c } else { a - b };

                self.config.sel.enable(&mut region, 0)?;

                region.assign_advice(|| "a", self.config.a, 0, || Value::known(a))?;
                region.assign_advice(|| "b", self.config.b, 0, || Value::known(b))?;
                region.assign_advice(|| "c", self.config.c, 0, || Value::known(c))?;
                region.assign_advice(
                    || "output",
                    self.config.output,
                    0,
                    || Value::known(output),
                )?;

                is_zero_chip.assign(&mut region, a - b)?;

                Ok(())
            },
        )
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
        let a = meta.advice_column();
        let b = meta.advice_column();
        let c = meta.advice_column();
        let output = meta.advice_column();
        let sel = meta.selector();

        let is_zero_config = IsZeroChip::configure(
            meta,
            |meta| meta.query_selector(sel),
            |meta| meta.query_advice(a, Rotation::cur()) - meta.query_advice(b, Rotation::cur()),
        );

        let foo_config = FooConfig {
            a,
            b,
            c,
            sel,
            output,
            is_zero_config,
        };

        FooChip::configure(meta, &foo_config);

        foo_config
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), plonk::Error> {
        let foo_chip = FooChip::new(config);

        foo_chip.assign(layouter.namespace(|| "assign"), &self.a, &self.b, &self.c)?;

        Ok(())
    }
}

fn main() {
    let k = 4;

    let circuit = FooCircuit {
        a: Some(Fp::from(11)),
        b: Some(Fp::from(11)),
        c: Some(Fp::from(22)),
    };

    let prover = MockProver::run(k, &circuit, vec![]).unwrap();
    prover.assert_satisfied();
}
