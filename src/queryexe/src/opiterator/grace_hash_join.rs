use std::collections::HashMap;

use common::{
    ids::{ContainerId, Permissions, StateType, TransactionId},
    query::bytecode_expr::ByteCodeExpr,
    traits::storage_trait::StorageTrait,
    CrustyError, Field, TableSchema, Tuple,
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use storage::StorageManager;

use crate::{opiterator::OpIterator, Managers};

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

    fn get_partition_id(key: &Field, num_partitions: usize) -> usize {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        (hasher.finish() as usize) % num_partitions
    }

    fn load_partition(&mut self, partition_id: usize) {
        // clear the prev hashmap
        self.current_join_map.clear();

        // load left partition from disk
        for (bytes, _) in self.managers.sm.get_iterator(
            self.left_partition_cids[partition_id],
            self.tid,
            Permissions::ReadOnly,
        ) {
            let t = Tuple::from_bytes(&bytes);
            let key = self.left_expr.eval(&t);
            self.current_join_map
                .entry(key)
                .or_default()
                .push(t);
        }

        // setup right partition iterator for probing
        self.current_right_iter = Some(self.managers.sm.get_iterator(
            self.right_partition_cids[partition_id],
            self.tid,
            Permissions::ReadOnly,
        ));

        // find first right tuple
        self.current_right_tuple = self
            .current_right_iter
            .as_mut()
            .and_then(|it| it.next())
            .map(|(bytes, _)| Tuple::from_bytes(&bytes));

        self.current_match_idx = 0;
    }
}

impl OpIterator for GraceHashJoin {
    fn configure(&mut self, will_rewind: bool) {
        self.left_child.configure(false);
        self.right_child.configure(will_rewind);
    }

    fn open(&mut self) -> Result<(), CrustyError> {
        if self.open {
            return Ok(());
        }
        self.right_child.open()?;
        self.left_child.open()?;

        // create containers to partition left and right table
        for i in 0..self.num_partitions {
            let left_cid = GRACE_BASE_CID + i as u16;
            let right_cid = GRACE_BASE_CID + self.num_partitions as u16 + i as u16;

            // left partitions
            self.managers
                .sm
                .create_container(left_cid, None, StateType::BaseTable, None)?;
            self.left_partition_cids.push(left_cid);

            // right
            self.managers
                .sm
                .create_container(right_cid, None, StateType::BaseTable, None)?;
            self.right_partition_cids.push(right_cid);
        }

        // hash and partition  left tuple keys to respective containers
        while let Some(t) = self.left_child.next()? {
            let key = self.left_expr.eval(&t);
            let pid = Self::get_partition_id(&key, self.num_partitions);
            self.managers
                .sm
                .insert_value(self.left_partition_cids[pid], t.to_bytes(), self.tid);
        }

        // hash and partition  right tuple keys to respective containers
        while let Some(t) = self.right_child.next()? {
            let key = self.right_expr.eval(&t);
            let pid = Self::get_partition_id(&key, self.num_partitions);
            self.managers
                .sm
                .insert_value(self.right_partition_cids[pid], t.to_bytes(), self.tid);
        }

        self.open = true;

        Ok(())
    }

    fn next(&mut self) -> Result<Option<Tuple>, CrustyError> {
        if !self.open {
            panic!("Iterator is not open");
        }

        // load partition from sm into a hash map
        if !self.partitions_loaded {
            self.load_partition(0);
            self.partitions_loaded = true;
        }

        // loop through partitions
        while self.current_partition < self.num_partitions {
            // probe hashtable with each container in the right partition to find matches
            while let Some(right_tuple) = self.current_right_tuple.clone() {
                let key = self.right_expr.eval(&right_tuple);
                if let Some(matches) = self.current_join_map.get(&key) {
                    if self.current_match_idx < matches.len() {
                        let left_tuple = matches[self.current_match_idx].clone();
                        self.current_match_idx += 1;
                        return Ok(Some(left_tuple.merge(&right_tuple)));
                    }
                }
                // exhausted matches, advance to next right child in partition
                self.current_right_tuple = self
                    .current_right_iter
                    .as_mut()
                    .and_then(|iter| iter.next())
                    .map(|(bytes, _)| Tuple::from_bytes(&bytes));
                self.current_match_idx = 0;
            }

            // advance to next partition
            self.current_partition += 1;
            if self.current_partition >= self.num_partitions {
                return Ok(None);
            }
            self.load_partition(self.current_partition);
        }

        Ok(None)
    }

    fn close(&mut self) -> Result<(), CrustyError> {
        self.left_child.close()?;
        self.right_child.close()?;
        self.current_join_map.clear();
        self.left_partition_cids.clear();
        self.right_partition_cids.clear();
        self.current_right_iter = None;
        self.current_right_tuple = None;
        self.current_partition = 0;
        self.current_match_idx = 0;
        self.partitions_loaded = false;
        self.open = false;
        Ok(())
    }

    fn rewind(&mut self) -> Result<(), CrustyError> {
        if !self.open {
            panic!("Operator has not been opened")
        }
        // reset probe state to partition 0
        self.current_partition = 0;
        self.current_join_map.clear();
        self.current_right_iter = None;
        self.current_right_tuple = None;
        self.current_match_idx = 0;
        // force to load partition on return
        self.partitions_loaded = false;
        Ok(())
    }

    fn get_schema(&self) -> &TableSchema {
        &self.schema
    }
}

#[cfg(test)]
mod test {
    use super::super::TupleIterator;
    use super::*;
    use crate::testutil::{execute_iter, new_test_managers, TestTuples};
    use common::query::bytecode_expr::{ByteCodeExpr, ByteCodes};
    use common::{Field, Tuple};

    fn get_join_predicate() -> (ByteCodeExpr, ByteCodeExpr) {
        // Joining two tables each containing the following tuples:
        // 1 1 3 E
        // 2 1 3 G
        // 3 1 4 A
        // 4 2 4 G
        // 5 2 5 G
        // 6 2 5 G
        // left(col(0) + col(1)) == right(col(2))
        let mut left = ByteCodeExpr::new();
        left.add_code(ByteCodes::PushField as usize);
        left.add_code(0);
        left.add_code(ByteCodes::PushField as usize);
        left.add_code(1);
        left.add_code(ByteCodes::Add as usize);

        let mut right = ByteCodeExpr::new();
        right.add_code(ByteCodes::PushField as usize);
        right.add_code(2);

        (left, right)
    }

    fn get_iter(left_expr: ByteCodeExpr, right_expr: ByteCodeExpr) -> Box<dyn OpIterator> {
        let setup = TestTuples::new("");
        let managers = new_test_managers();
        let mut iter = Box::new(GraceHashJoin::new(
            managers,
            setup.schema.clone(),
            left_expr,
            right_expr,
            Box::new(TupleIterator::new(
                setup.tuples.clone(),
                setup.schema.clone(),
            )),
            Box::new(TupleIterator::new(
                setup.tuples.clone(),
                setup.schema.clone(),
            )),
            Some(4),
            TransactionId::new(),
        ));
        iter.configure(false);
        iter
    }

    mod grace_hash_join_test {
        use super::*;

        #[test]
        #[should_panic]
        fn test_empty_predicate_join() {
            let left_expr = ByteCodeExpr::new();
            let right_expr = ByteCodeExpr::new();
            let mut iter = get_iter(left_expr, right_expr);
            let _ = execute_iter(&mut *iter, true).unwrap();
        }

        #[test]
        fn test_join() {
            // Input (both sides):
            // 1 1 3 E
            // 2 1 3 G
            // 3 1 4 A
            // 4 2 4 G
            // 5 2 5 G
            // 6 2 5 G
            // Predicate: left(col0 + col1) == right(col2)
            // Matches:
            //   left(2,1,3,G): 2+1=3 → right(1,1,3,E) and right(2,1,3,G)
            //   left(3,1,4,A): 3+1=4 → right(3,1,4,A) and right(4,2,4,G)
            let (left_expr, right_expr) = get_join_predicate();
            let mut iter = get_iter(left_expr, right_expr);
            let t = execute_iter(&mut *iter, true).unwrap();
            assert_eq!(t.len(), 4);
            assert_eq!(
                t[0],
                Tuple::new(vec![
                    Field::BigInt(2),
                    Field::BigInt(1),
                    Field::BigInt(3),
                    Field::String("G".to_string()),
                    Field::BigInt(1),
                    Field::BigInt(1),
                    Field::BigInt(3),
                    Field::String("E".to_string()),
                ])
            );
            assert_eq!(
                t[1],
                Tuple::new(vec![
                    Field::BigInt(2),
                    Field::BigInt(1),
                    Field::BigInt(3),
                    Field::String("G".to_string()),
                    Field::BigInt(2),
                    Field::BigInt(1),
                    Field::BigInt(3),
                    Field::String("G".to_string()),
                ])
            );
            assert_eq!(
                t[2],
                Tuple::new(vec![
                    Field::BigInt(3),
                    Field::BigInt(1),
                    Field::BigInt(4),
                    Field::String("A".to_string()),
                    Field::BigInt(3),
                    Field::BigInt(1),
                    Field::BigInt(4),
                    Field::String("A".to_string()),
                ])
            );
            assert_eq!(
                t[3],
                Tuple::new(vec![
                    Field::BigInt(3),
                    Field::BigInt(1),
                    Field::BigInt(4),
                    Field::String("A".to_string()),
                    Field::BigInt(4),
                    Field::BigInt(2),
                    Field::BigInt(4),
                    Field::String("G".to_string()),
                ])
            );
        }
    }

    mod opiterator_test {
        use super::*;

        #[test]
        #[should_panic]
        fn test_next_not_open() {
            let (left_expr, right_expr) = get_join_predicate();
            let mut iter = get_iter(left_expr, right_expr);
            let _ = iter.next();
        }

        #[test]
        #[should_panic]
        fn test_rewind_not_open() {
            let (left_expr, right_expr) = get_join_predicate();
            let mut iter = get_iter(left_expr, right_expr);
            let _ = iter.rewind();
        }

        #[test]
        fn test_open() {
            let (left_expr, right_expr) = get_join_predicate();
            let mut iter = get_iter(left_expr, right_expr);
            iter.open().unwrap();
        }

        #[test]
        fn test_close() {
            let (left_expr, right_expr) = get_join_predicate();
            let mut iter = get_iter(left_expr, right_expr);
            iter.open().unwrap();
            iter.close().unwrap();
        }

        #[test]
        fn test_rewind() {
            let (left_expr, right_expr) = get_join_predicate();
            let mut iter = get_iter(left_expr, right_expr);
            iter.configure(true);
            let t_before = execute_iter(&mut *iter, true).unwrap();
            iter.rewind().unwrap();
            let t_after = execute_iter(&mut *iter, true).unwrap();
            assert_eq!(t_before, t_after);
        }
    }
}
