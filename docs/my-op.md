# Write up

Adithya Suresh

## Query Life-Cycle Question
### SELECT * FROM table WHERE a > 10: ###
1. recieve SQL query on server.rs
2. SQL parser produces an AST
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