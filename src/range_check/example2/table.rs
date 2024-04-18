use std::marker::PhantomData;

use halo2_proofs::{
    arithmetic::Field,
    circuit::{Layouter, Value},
    plonk::{self, ConstraintSystem, TableColumn},
};

#[derive(Clone)]
pub struct RangeCheckTable<F, const RANGE: usize> {
    col: TableColumn,
    _marker: PhantomData<F>,
}

impl<F: Field, const RANGE: usize> RangeCheckTable<F, RANGE> {
    pub fn new(meta: &mut ConstraintSystem<F>) -> Self {
        let col = meta.lookup_table_column();
        Self {
            col,
            _marker: PhantomData,
        }
    }

    pub fn table_column(&self) -> &TableColumn {
        &self.col
    }

    pub fn load(&self, mut layouter: impl Layouter<F>) -> Result<(), plonk::Error> {
        layouter.assign_table(
            || "load table",
            |mut table| {
                let mut value = F::ZERO;
                for i in 0..RANGE {
                    table.assign_cell(
                        || "assign table cell",
                        self.col,
                        i,
                        || Value::known(value),
                    )?;

                    value += F::ONE;
                }

                Ok(())
            },
        )
    }
}
