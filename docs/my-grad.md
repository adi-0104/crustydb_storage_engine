# Write up

Adithya Suresh

##  Why Grace Hash Join

The existing `HashEqJoin` loads the entire left (build) side into an in-memory `HashMap`. For large tables this causes out-of-memory failures. Grace Hash Join solves this by partitioning both input sides to disk first, then joining one partition pair at a time. this bounds memory usage to the size of one partition instead of the entire table.

## Design

### Algorithm

Grace Hash Join runs in two phases:

**Phase 1 — Partition (in `open()`)**
Both the left and right children are fully consumed. Each tuple's join key is evaluated and hashed to a partition id: `hash(key) % num_partitions`. Tuples are serialized with `Tuple::to_bytes()` and written to temporary disk containers via `managers.sm.insert_value()`. Left tuples go to `left_partition[pid]`, right tuples to `right_partition[pid]`. So any two tuples that satisfy the join condition will always hash to the same partition index on both sides, so cross-partition joins are never needed.

**Phase 2 — Build & Probe (in `next()`)**

Build: primarily hosted in `load_partition(self, pid)` Partitions are processed one at a time. For partition `i`, all left tuples are read from disk via `managers.sm.get_iterator()` and loaded into an a `current_join_map<Field, Vec<Tuple>>` .

Probe:  right tuples from partition `i` are iterated one at a time via a disk iterator. Each right tuple's join key is probed against the map matching left tuples are merged and returned one per `next()` call using `current_match_idx`. When the right partition is exhausted, we move to the next partition and repeat.

### States

```
left_partition_cids / right_partition_cids  — container IDs for each partition on disk
current_partition — which partition pair we are currently joining
current_join_map — in-memory build side (HashMap) for the current partition
current_right_iter — disk iterator for the current right partition
current_right_tuple — current probe tuple being matched against the build side
current_match_idx — position within the Vec of matching left tuples
partitions_loaded — flag to trigger loading partition 0 on the first call to next()
```

## Testing

Tests mirror the structure of `hash_join.rs` and `nested_loop_join.rs`:

Run with:
```bash
cargo test -p queryexe -- grace_hash_join
```

## Time Estimate and Reflection

~12 hours

The hardest part was `next()` — managing the state machine across partition rather than the right child iter that we exhausted in `open()` 

## References

- Ramakrishnan & Gehrke, *Database Management Systems* Ch. 14 — External Hashing & Grace Hash Join
- https://www.youtube.com/watch?v=GRONctC_Uh0
- Rust `std::hash` docs: https://doc.rust-lang.org/std/hash/index.html
- Existing `hash_join.rs` and `nested_loop_join.rs` for operator structure and test patterns
- Rust `and_then` docs:  https://doc.rust-lang.org/rust-by-example/error/option_unwrap/and_then.html
