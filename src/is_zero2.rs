use std::marker::PhantomData;

use halo2_proofs::{
    arithmetic::Field,
    circuit::{Region, Value},
    plonk::{self, Advice, Column, ConstraintSystem, Constraints, Expression, VirtualCells},
    poly::Rotation,
};

#[derive(Clone)]
pub struct IsZero2Config<F> {
    value_inv: Column<Advice>,
    is_zero_expr: Expression<F>,
}

impl<F: Field> IsZero2Config<F> {
    pub fn expr(&self) -> &Expression<F> {
        &self.is_zero_expr
    }
}

pub struct IsZero2Chip<F> {
    config: IsZero2Config<F>,
    _marker: PhantomData<F>,
}

impl<F: Field> IsZero2Chip<F> {
    pub fn new(config: IsZero2Config<F>) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }

    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        g_sel: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
        value: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) -> IsZero2Config<F> {
        let value_inv = meta.advice_column();
        let mut is_zero_expr = Expression::Constant(F::ZERO);

        meta.create_gate("is zero", |meta| {
            let g_sel = g_sel(meta);
            let value = value(meta);

            let value_inv = meta.query_advice(value_inv, Rotation::cur());

            is_zero_expr = Expression::Constant(F::ONE) - value.clone() * value_inv;
            Constraints::with_selector(g_sel, [value * is_zero_expr.clone()])
        });

        IsZero2Config {
            value_inv,
            is_zero_expr,
        }
    }

    pub fn assign(&self, region: &mut Region<'_, F>, value: F) -> Result<(), plonk::Error> {
        region.assign_advice(
            || "value inv",
            self.config.value_inv,
            0,
            || Value::known(value),
        )?;
        Ok(())
    }
}
