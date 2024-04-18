use halo2_proofs::{
    arithmetic::Field,
    circuit::{Layouter, Value},
    plonk::{self, Advice, Column, ConstraintSystem, Constraints, Expression, Selector},
    poly::Rotation,
};

use self::table::RangeCheckTable;

mod table;

#[derive(Clone)]
pub struct RangeCheckConfig<F, const RANGE: usize> {
    q_check: Selector,
    q_table: Selector,

    value_col: Column<Advice>,

    range_check_table: RangeCheckTable<F, RANGE>,
}

pub struct RangeCheckChip<F, const RANGE: usize> {
    config: RangeCheckConfig<F, RANGE>,
}

impl<F: Field, const RANGE: usize> RangeCheckChip<F, RANGE> {
    pub fn new(config: RangeCheckConfig<F, RANGE>) -> Self {
        Self { config }
    }

    pub fn configure(meta: &mut ConstraintSystem<F>) -> RangeCheckConfig<F, RANGE> {
        let q_check = meta.selector();
        let q_table = meta.complex_selector();
        let value_col = meta.advice_column();

        let range_check_table = RangeCheckTable::<F, RANGE>::new(meta);

        meta.create_gate("range check", |meta| {
            let sel = meta.query_selector(q_check);
            let value = meta.query_advice(value_col, Rotation::cur());

            let (range_check, _) =
                (0..RANGE)
                    .into_iter()
                    .fold((value.clone(), F::ZERO), |(acc, v), _| {
                        let v = v + F::ONE;
                        (acc * (value.clone() - Expression::Constant(v)), v)
                    });

            Constraints::with_selector(sel, [range_check])
        });

        meta.lookup("range lookup", |table| {
            let sel = table.query_selector(q_table);
            let value = table.query_advice(value_col, Rotation::cur());

            vec![(sel * value, range_check_table.table_column().clone())]
        });

        RangeCheckConfig {
            q_check,
            q_table,
            value_col,
            range_check_table,
        }
    }

    pub fn load_table(&self, layouter: impl Layouter<F>) -> Result<(), plonk::Error> {
        self.config.range_check_table.load(layouter)
    }

    pub fn assign(
        &self,
        mut layouter: impl Layouter<F>,
        value: Option<F>,
    ) -> Result<(), plonk::Error> {
        layouter.assign_region(
            || "assign region",
            |mut region| {
                if RANGE <= 256 {
                    self.config.q_check.enable(&mut region, 0)?;
                } else {
                    self.config.q_table.enable(&mut region, 0)?;
                }

                let value = value.ok_or(plonk::Error::Synthesis)?;
                region.assign_advice(
                    || "assign",
                    self.config.value_col,
                    0,
                    || Value::known(value),
                )?;
                Ok(())
            },
        )
    }
}
