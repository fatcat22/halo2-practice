use halo2_proofs::{
    arithmetic::Field,
    circuit::{Region, Value},
    plonk::{self, Advice, Column, ConstraintSystem, Expression, VirtualCells},
    poly::Rotation,
};

#[derive(Clone)]
pub struct IsZeroConfig<F> {
    value_inv: Column<Advice>,
    is_zero_expr: Expression<F>,
}

impl<F: Field> IsZeroConfig<F> {
    pub fn expr(&self) -> Expression<F> {
        self.is_zero_expr.clone()
    }
}

pub struct IsZeroChip<F> {
    config: IsZeroConfig<F>,
}

impl<F: Field> IsZeroChip<F> {
    pub fn new(config: IsZeroConfig<F>) -> Self {
        Self { config }
    }

    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        g_sel: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
        value: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) -> IsZeroConfig<F> {
        let mut is_zero_expr = Expression::Constant(F::ZERO);

        let value_inv = meta.advice_column();

        meta.create_gate("is zero", |meta| {
            let value_inv = meta.query_advice(value_inv, Rotation::cur());

            let g_sel = g_sel(meta);
            let value = value(meta);

            // value | value_rev | 1 - value * value_rev
            //   x   |    1/x    |   0
            //   x   |    0      |   1
            //   0   |    0      |   1
            //   0   |    y      |   1

            is_zero_expr = Expression::Constant(F::ONE) - value.clone() * value_inv;

            vec![g_sel * value * is_zero_expr.clone()]
        });

        IsZeroConfig {
            value_inv,
            is_zero_expr,
        }
    }

    pub fn assign(&self, region: &mut Region<'_, F>, value: F) -> Result<(), plonk::Error> {
        let value_inv = Value::known(value.invert().unwrap_or(F::ZERO));
        region.assign_advice(|| "assign invert", self.config.value_inv, 0, || value_inv)?;
        Ok(())
    }
}
