use halo2_proofs::{
    plonk::{Advice, Column, ConstraintSystem, Expression, Fixed},
    poly::Rotation,
};
use eth_types::Field;
use std::marker::PhantomData;

use crate::{
    helpers::get_is_extension_node,
    param::{KECCAK_INPUT_WIDTH, KECCAK_OUTPUT_WIDTH, IS_BRANCH_S_PLACEHOLDER_POS, IS_BRANCH_C_PLACEHOLDER_POS, RLP_NUM}, mpt::MainCols,
};

#[derive(Clone, Debug)]
pub(crate) struct BranchHashInParentConfig<F> {
    _marker: PhantomData<F>,
}

impl<F: Field> BranchHashInParentConfig<F> {
    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        inter_root: Column<Advice>,
        not_first_level: Column<Advice>,
        q_not_first: Column<Fixed>,
        is_account_leaf_in_added_branch: Column<Advice>,
        is_last_branch_child: Column<Advice>,
        s_main: MainCols,
        mod_node_hash_rlc: Column<Advice>,
        acc: Column<Advice>,
        acc_mult: Column<Advice>,
        keccak_table: [Column<Fixed>; KECCAK_INPUT_WIDTH + KECCAK_OUTPUT_WIDTH],
        is_s: bool,
    ) -> Self {
        let config = BranchHashInParentConfig { _marker: PhantomData, }; 
        let one = Expression::Constant(F::from(1_u64));

        meta.lookup_any(
            "account first level branch hash - compared to root",
            |meta| {
                let mut constraints = vec![];
                let q_not_first = meta.query_fixed(q_not_first, Rotation::cur());
                let not_first_level = meta.query_advice(not_first_level, Rotation::cur());

                let is_last_branch_child = meta.query_advice(is_last_branch_child, Rotation::cur());

                // TODO: acc currently doesn't have branch ValueNode info (which 128 if nil)
                let acc = meta.query_advice(acc, Rotation::cur());
                let c128 = Expression::Constant(F::from(128));
                let mult = meta.query_advice(acc_mult, Rotation::cur());
                let branch_acc = acc + c128 * mult;

                let root = meta.query_advice(inter_root, Rotation::cur());

                constraints.push((
                    q_not_first.clone()
                        * is_last_branch_child.clone()
                        * (one.clone() - not_first_level.clone())
                        * branch_acc, // TODO: replace with acc once ValueNode is added
                    meta.query_fixed(keccak_table[0], Rotation::cur()),
                ));
                let keccak_table_i = meta.query_fixed(keccak_table[1], Rotation::cur());
                constraints.push((
                    q_not_first * is_last_branch_child * (one.clone() - not_first_level) * root,
                    keccak_table_i,
                ));

                constraints
            },
        );

        // Check whether hash of a branch is in parent branch.
        // Check if (accumulated_s(c)_rlc, hash1, hash2, hash3, hash4) is in keccak
        // table, where hash1, hash2, hash3, hash4 are stored in the previous
        // branch and accumulated_s(c)_rlc presents the branch RLC.
        meta.lookup_any("branch_hash_in_parent", |meta| {
            let not_first_level = meta.query_advice(not_first_level, Rotation::cur());

            // -17 because we are in the last branch child (-16 takes us to branch init)
            let is_account_leaf_in_added_branch_prev =
                meta.query_advice(is_account_leaf_in_added_branch, Rotation(-17));

            // We need to do the lookup only if we are in the last branch child.
            let is_last_branch_child = meta.query_advice(is_last_branch_child, Rotation::cur());

            // When placeholder branch, we don't check its hash in a parent.
            let mut is_branch_placeholder = meta.query_advice(s_main.bytes[IS_BRANCH_S_PLACEHOLDER_POS - RLP_NUM], Rotation(-16));
            if !is_s {
                is_branch_placeholder = meta.query_advice(s_main.bytes[IS_BRANCH_C_PLACEHOLDER_POS - RLP_NUM], Rotation(-16));
            }

            let is_extension_node = get_is_extension_node(meta, s_main.bytes, -16);

            // TODO: acc currently doesn't have branch ValueNode info (which 128 if nil)
            let acc = meta.query_advice(acc, Rotation::cur());
            let c128 = Expression::Constant(F::from(128));
            let mult = meta.query_advice(acc_mult, Rotation::cur());
            let branch_acc = acc + c128 * mult;

            let mut constraints = vec![(
                not_first_level.clone()
                    * is_last_branch_child.clone()
                    * (one.clone() - is_account_leaf_in_added_branch_prev.clone()) // we don't check this in the first storage level
                    * (one.clone() - is_branch_placeholder.clone())
                    * (one.clone() - is_extension_node.clone())
                    * branch_acc, // TODO: replace with acc once ValueNode is added
                meta.query_fixed(keccak_table[0], Rotation::cur()),
            )];
            // Any rotation that lands into branch can be used instead of -19.
            let mod_node_hash_rlc_cur = meta.query_advice(mod_node_hash_rlc, Rotation(-19));
            let keccak_table_i = meta.query_fixed(keccak_table[1], Rotation::cur());
            constraints.push((
                not_first_level
                        * is_last_branch_child
                        * (one.clone()
                            - is_account_leaf_in_added_branch_prev) // we don't check this in the first storage level
                        * (one.clone() - is_branch_placeholder)
                        * (one.clone() - is_extension_node)
                        * mod_node_hash_rlc_cur,
                keccak_table_i,
            ));

            constraints
        });

        config
    }
}