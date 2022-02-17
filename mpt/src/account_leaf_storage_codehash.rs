use halo2::{
    circuit::Chip,
    plonk::{
        Advice, Column, ConstraintSystem, Expression, Fixed, VirtualCells,
    },
    poly::Rotation,
};
use pairing::arithmetic::FieldExt;
use std::marker::PhantomData;

use crate::{helpers::range_lookups, mpt::FixedTableTag, param::HASH_WIDTH};

#[derive(Clone, Debug)]
pub(crate) struct AccountLeafStorageCodehashConfig {}

// Verifies the hash of a leaf is in the parent branch.
pub(crate) struct AccountLeafStorageCodehashChip<F> {
    config: AccountLeafStorageCodehashConfig,
    _marker: PhantomData<F>,
}

impl<F: FieldExt> AccountLeafStorageCodehashChip<F> {
    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        q_enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F> + Copy,
        s_rlp2: Column<Advice>,
        c_rlp2: Column<Advice>,
        s_advices: [Column<Advice>; HASH_WIDTH],
        c_advices: [Column<Advice>; HASH_WIDTH],
        acc_r: F,
        acc: Column<Advice>,
        acc_mult: Column<Advice>,
        fixed_table: [Column<Fixed>; 3],
        is_s: bool,
    ) -> AccountLeafStorageCodehashConfig {
        let config = AccountLeafStorageCodehashConfig {};

        // We don't need to check acc_mult because it's not used after this row.

        meta.create_gate("account leaf storage codehash", |meta| {
            let q_enable = q_enable(meta);
            let mut constraints = vec![];

            // TODO: RLP properties

            // We have storage length in s_rlp2 (which is 160 presenting 128 + 32).
            // We have storage hash in s_advices.
            // We have codehash length in c_rlp2 (which is 160 presenting 128 + 32).
            // We have codehash in c_advices.

            let c160 = Expression::Constant(F::from(160));
            let mut rot = -1;
            if !is_s {
                rot = -2;
            }
            let acc_prev = meta.query_advice(acc, Rotation(rot));
            let acc_mult_prev = meta.query_advice(acc_mult, Rotation(rot));
            let mut curr_r = acc_mult_prev;
            let s_rlp2 = meta.query_advice(s_rlp2, Rotation::cur());
            let c_rlp2 = meta.query_advice(c_rlp2, Rotation::cur());
            constraints.push((
                "account leaf storage codehash s_rlp2",
                q_enable.clone() * (s_rlp2.clone() - c160.clone()),
            ));
            constraints.push((
                "account leaf storage codehash c_rlp2",
                q_enable.clone() * (c_rlp2.clone() - c160),
            ));

            let mut expr = acc_prev + s_rlp2 * curr_r.clone();
            curr_r = curr_r * acc_r;
            for col in s_advices.iter() {
                let s = meta.query_advice(*col, Rotation::cur());
                expr = expr + s * curr_r.clone();
                curr_r = curr_r * acc_r;
            }

            expr = expr + c_rlp2 * curr_r.clone();
            curr_r = curr_r * acc_r;
            for col in c_advices.iter() {
                let c = meta.query_advice(*col, Rotation::cur());
                expr = expr + c * curr_r.clone();
                curr_r = curr_r * acc_r;
            }

            let acc = meta.query_advice(acc, Rotation::cur());
            constraints.push((
                "account leaf storage codehash acc",
                q_enable * (expr - acc),
            ));

            constraints
        });

        range_lookups(
            meta,
            q_enable.clone(),
            s_advices.to_vec(),
            FixedTableTag::Range256,
            fixed_table,
        );
        range_lookups(
            meta,
            q_enable.clone(),
            c_advices.to_vec(),
            FixedTableTag::Range256,
            fixed_table,
        );
        // s_rlp1 and c_rlp1 not used
        range_lookups(
            meta,
            q_enable,
            [s_rlp2, c_rlp2].to_vec(),
            FixedTableTag::Range256,
            fixed_table,
        );

        config
    }

    pub fn construct(config: AccountLeafStorageCodehashConfig) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }
}

impl<F: FieldExt> Chip<F> for AccountLeafStorageCodehashChip<F> {
    type Config = AccountLeafStorageCodehashConfig;
    type Loaded = ();

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn loaded(&self) -> &Self::Loaded {
        &()
    }
}
