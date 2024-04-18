use std::marker::PhantomData;

use halo2_proofs::{
    arithmetic::Field,
    circuit::{Layouter, Value},
    plonk::{self, Advice, Column, ConstraintSystem, Constraints, Expression, Selector},
    poly::Rotation,
};

#[derive(Clone)]
pub struct RangeCheckConfig {
    value: Column<Advice>,
    sel: Selector,
}

pub struct RangeCheckChip<F, const RANGE: usize> {
    config: RangeCheckConfig,
    _marker: PhantomData<F>,
}

impl<F: Field, const RANGE: usize> RangeCheckChip<F, RANGE> {
    pub fn new(config: RangeCheckConfig) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }

    pub fn configure(meta: &mut ConstraintSystem<F>) -> RangeCheckConfig {
        let value = meta.advice_column();
        let sel = meta.selector();

        let config = RangeCheckConfig { value, sel };

        meta.create_gate("range check", |meta| {
            let sel = meta.query_selector(config.sel);
            let value = meta.query_advice(config.value, Rotation::cur());

            let (range_check, _) =
                (0..(RANGE - 1))
                    .into_iter()
                    .fold((value.clone(), F::ZERO), |(acc, v), _| {
                        let v = v + F::ONE;
                        (acc * (value.clone() - Expression::Constant(v)), v)
                    });

            Constraints::with_selector(sel, [range_check])
        });

        config
    }

    pub fn assign(&self, mut layouter: impl Layouter<F>, v: Option<F>) -> Result<(), plonk::Error> {
        layouter.assign_region(
            || "assign value",
            |mut region| {
                self.config.sel.enable(&mut region, 0)?;

                let v = v.ok_or(plonk::Error::Synthesis)?;
                region.assign_advice(|| "value", self.config.value, 0, || Value::known(v))?;

                Ok(())
            },
        )
    }
}
