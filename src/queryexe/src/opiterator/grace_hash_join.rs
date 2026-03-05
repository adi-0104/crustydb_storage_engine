use std::{collections::HashMap};

use common::{Field, TableSchema, Tuple, ids::{ContainerId, TransactionId}, query::bytecode_expr::ByteCodeExpr, traits::storage_trait::StorageTrait, CrustyError, ids::StateType};
use storage::StorageManager;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::{Managers, opiterator::OpIterator};

pub struct GraceHashJoin {
    #[allow(dead_code)]
    // Static
    managers: &'static Managers,

    // Parameters (No need to reset on close)
    schema: TableSchema,
    left_expr: ByteCodeExpr,
    right_expr: ByteCodeExpr,
    left_child: Box<dyn OpIterator>,
    right_child: Box<dyn OpIterator>,
    num_partitions: usize,
    tid: TransactionId,

    // States (reset on close)
    open: bool,
    left_partition_cids: Vec<ContainerId>,
    right_partition_cids: Vec<ContainerId>,
    current_partition: usize,
    current_join_map: HashMap<Field, Vec<Tuple>>,
    current_right_iter: Option<<StorageManager as StorageTrait>::ValIterator>,
    current_right_tuple: Option<Tuple>,
    current_match_idx: usize,
    partitions_loaded: bool,
}
const GRACE_BASE_CID: ContainerId = 40000;

impl GraceHashJoin {
    /// Constructor for a grace hash equi-join operator.
    ///
    /// # Arguments
    ///TODO doc string
    ///
    ///
    ///
    ///
    pub fn new(
        managers: &'static Managers,
        schema: TableSchema,
        left_expr: ByteCodeExpr,
        right_expr: ByteCodeExpr,
        left_child: Box<dyn OpIterator>,
        right_child: Box<dyn OpIterator>,
        num_partitions: Option<usize>,
        tid: TransactionId,
    ) -> Self {
        Self {
            managers,
            schema,
            left_expr,
            right_expr,
            left_child,
            right_child,
            num_partitions: num_partitions.unwrap_or(10),
            tid,
            open: false,
            left_partition_cids: Vec::new(),
            right_partition_cids: Vec::new(),
            current_partition: 0,
            current_join_map: HashMap::new(),
            current_right_iter: None,
            current_right_tuple: None,
            current_match_idx: 0,
            partitions_loaded: false,
        }
    }

}
