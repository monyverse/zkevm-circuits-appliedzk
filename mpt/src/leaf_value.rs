use halo2::{
    circuit::Chip,
    plonk::{
        Advice, Column, ConstraintSystem, Expression, Fixed, VirtualCells,
    },
    poly::Rotation,
};
use pairing::arithmetic::FieldExt;
use std::marker::PhantomData;

use crate::{
    helpers::range_lookups,
    mpt::FixedTableTag,
    param::{HASH_WIDTH, KECCAK_INPUT_WIDTH, KECCAK_OUTPUT_WIDTH},
};

#[derive(Clone, Debug)]
pub(crate) struct LeafValueConfig {}

// Verifies the hash of a leaf is in the parent branch.
pub(crate) struct LeafValueChip<F> {
    config: LeafValueConfig,
    _marker: PhantomData<F>,
}

impl<F: FieldExt> LeafValueChip<F> {
    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        q_enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F> + Copy,
        s_rlp1: Column<Advice>,
        s_rlp2: Column<Advice>,
        s_advices: [Column<Advice>; HASH_WIDTH],
        sc_keccak: [Column<Advice>; KECCAK_OUTPUT_WIDTH],
        keccak_table: [Column<Fixed>; KECCAK_INPUT_WIDTH + KECCAK_OUTPUT_WIDTH],
        acc: Column<Advice>,
        acc_mult: Column<Advice>,
        sel: Column<Advice>,
        is_account_leaf_storage_codehash_c: Column<Advice>,
        is_branch_placeholder: Column<Advice>,
        is_s: bool,
        acc_r: F,
        fixed_table: [Column<Fixed>; 3],
    ) -> LeafValueConfig {
        let config = LeafValueConfig {};

        // TODO: use r_table

        // NOTE: Rotation -6 can be used here (in S and C leaf), because
        // s_keccak and c_keccak have the same value in all branch rows (thus, the same
        // value in branch node_index: 13 and branch node_index: 15).
        // The same holds for sel1 and sel2.
        let rot = -6;
        let mut rot_into_init = -20;
        if !is_s {
            rot_into_init = -22;
        }

        meta.lookup_any(|meta| {
            let q_enable = q_enable(meta);

            let mut rlc = meta.query_advice(acc, Rotation::prev());
            let mut mult = meta.query_advice(acc_mult, Rotation::prev());

            let s_rlp1 = meta.query_advice(s_rlp1, Rotation::cur());
            rlc = rlc + s_rlp1 * mult.clone();
            mult = mult * acc_r;

            let s_rlp2 = meta.query_advice(s_rlp2, Rotation::cur());
            rlc = rlc + s_rlp2 * mult.clone();
            mult = mult * acc_r;

            for col in s_advices.iter() {
                let s = meta.query_advice(*col, Rotation::cur());
                rlc = rlc + s * mult.clone();
                mult = mult * acc_r;
            }

            let sel = meta.query_advice(sel, Rotation(rot));
            let one = Expression::Constant(F::one());

            let is_branch_placeholder = meta
                .query_advice(is_branch_placeholder, Rotation(rot_into_init));

            let mut rot_leaf = -1;
            if !is_s {
                rot_leaf = -2;
            }
            // For leaf without branch, the constraints are in storage_root_in_account_leaf.
            let is_leaf_without_branch = meta.query_advice(
                is_account_leaf_storage_codehash_c,
                Rotation(rot_leaf),
            );

            // If sel = 1, there is no leaf at this position (value is being added or deleted)
            // and we don't check the hash of it.
            let mut constraints = vec![];
            constraints.push((
                q_enable.clone()
                    * rlc
                    * (one.clone() - sel.clone())
                    * (one.clone() - is_leaf_without_branch.clone())
                    * (one.clone() - is_branch_placeholder.clone()),
                meta.query_fixed(keccak_table[0], Rotation::cur()),
            ));
            for (ind, column) in sc_keccak.iter().enumerate() {
                let sc_keccak = meta.query_advice(*column, Rotation(rot));
                let keccak_table_i =
                    meta.query_fixed(keccak_table[ind + 1], Rotation::cur());
                constraints.push((
                    q_enable.clone()
                        * sc_keccak
                        * (one.clone() - sel.clone())
                        * (one.clone() - is_leaf_without_branch.clone())
                        * (one.clone() - is_branch_placeholder.clone()),
                    keccak_table_i,
                ));
            }

            constraints
        });

        // Lookup for case when there is a placeholder branch - in this case we need to check
        // the hash in the branch above the placeholder branch.
        meta.lookup_any(|meta| {
            let q_enable = q_enable(meta);

            let mut rlc = meta.query_advice(acc, Rotation::prev());
            let mut mult = meta.query_advice(acc_mult, Rotation::prev());

            let s_rlp1 = meta.query_advice(s_rlp1, Rotation::cur());
            rlc = rlc + s_rlp1 * mult.clone();
            mult = mult * acc_r;

            let s_rlp2 = meta.query_advice(s_rlp2, Rotation::cur());
            rlc = rlc + s_rlp2 * mult.clone();
            mult = mult * acc_r;

            for col in s_advices.iter() {
                let s = meta.query_advice(*col, Rotation::cur());
                rlc = rlc + s * mult.clone();
                mult = mult * acc_r;
            }

            let sel = meta.query_advice(sel, Rotation(rot));
            let one = Expression::Constant(F::one());

            let is_branch_placeholder = meta
                .query_advice(is_branch_placeholder, Rotation(rot_into_init));

            // For leaf without branch, the constraints are in storage_root_in_account_leaf.
            let is_leaf_without_branch = meta.query_advice(
                is_account_leaf_storage_codehash_c,
                Rotation(rot_into_init - 1),
            );

            // Note: sel1 and sel2 in branch children: denote whether there is no leaf at is_modified (when value is added or deleted from trie)

            // If sel = 1, there is no leaf at this position (value is being added or deleted)
            // and we don't check the hash of it.
            let mut constraints = vec![];
            constraints.push((
                q_enable.clone()
                    * rlc
                    * (one.clone() - sel.clone())
                    * (one.clone() - is_leaf_without_branch.clone())
                    * is_branch_placeholder.clone(),
                meta.query_fixed(keccak_table[0], Rotation::cur()),
            ));
            for (ind, column) in sc_keccak.iter().enumerate() {
                let sc_keccak = meta.query_advice(
                    *column,
                    Rotation(rot_into_init - 3), // -3 to get from init branch into the previous branch (last row), note that -2 is needed because of extension nodes
                );
                let keccak_table_i =
                    meta.query_fixed(keccak_table[ind + 1], Rotation::cur());
                constraints.push((
                    q_enable.clone()
                        * sc_keccak
                        * (one.clone() - sel.clone())
                        * (one.clone() - is_leaf_without_branch.clone())
                        * is_branch_placeholder.clone(),
                    keccak_table_i,
                ));
            }

            constraints
        });

        range_lookups(
            meta,
            q_enable,
            s_advices.to_vec(),
            FixedTableTag::Range256,
            fixed_table,
        );
        range_lookups(
            meta,
            q_enable,
            [s_rlp1, s_rlp2].to_vec(),
            FixedTableTag::Range256,
            fixed_table,
        );

        config
    }

    pub fn construct(config: LeafValueConfig) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }
}

impl<F: FieldExt> Chip<F> for LeafValueChip<F> {
    type Config = LeafValueConfig;
    type Loaded = ();

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn loaded(&self) -> &Self::Loaded {
        &()
    }
}
