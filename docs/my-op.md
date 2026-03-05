# Write up

Adithya Suresh

## Query Life-Cycle Question
### SELECT * FROM table WHERE a > 10: ###
1. The client sends the SQL string to server.rs (server.rs receives it)
2. The server parses the SQL into an AST
3. That tree gets converted into a logical plan — a high level description 
   of what to do using the translate_and_validate.rs file
    From - LogicalRelExpr::Scan{}
    Where a > 10 - LogicalRelExpr::Select {}

4. The logical plan gets converted into a physical plan,
MockOptimizer calls logical_rel_expr.to_physical_plan() which has a mapping with the logical plan (LogicalRelExpr - PhysicalRelExpr)

5. The planner.rs file builds the operator objects by walking through the physical plan recursively and instantiates OpIterator Objects

6. The executor runs the pipeline using the Volcano model by first calling open() and then repeatedly calls next() on Filter which calls next() on sSeqScan:

7. In SeqScan we call the storage manager uses get_iterator which returns a HeapFileIter
8. The HeapFileIter iterates through pages of the heapfile fetching each page via the buffer pool
## Design
NestedLoopJoin:
Follows the Volcano model. current_tuple holds the current left tuple while the right 
child is iterated fully. When the right is exhausted it is rewound and the next left 
tuple is fetched.

HashEqJoin:
divided by the build phase and a probe phase
Open() handles the build phase where a hashmap is created using the tuples in the left table.
Next() - here we probe the right tuple values in the hash map and use current_idx to track position within that hash. 

Aggregate:
performs a fully blocking operation. consumes all child tuples before next() can return anything. Supports multiple GROUP BY and multiple aggregate columns via Vec.
merge_tuple_into_group() initializes new groups directly (COUNT starts at BigInt(1), 
others use the first field value).
AVG stores a running sum and divides when building acc_iter.


## Time Estimate / Reflection 
7hrs


## References

HashMaps methods in rust: https://doc.rust-lang.org/rust-by-example/std/hash.html. and some LLM queries to see valid examples of using methods like get_mut, or_insert()