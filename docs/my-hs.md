# Write up

Adithya Suresh

## Design

Main design decisions apart from the implementation details:
Added a Dirty Page Marker in the heap_page header metadata so that inserting a value can skip the linear scan for deleted slots to reuse until a delete has actually occurred on that page.

add_val - does a read lock pass first to skip attempts at writing to pages that don't have space for the slot metadata and actual value.


## Time Estimate / Reflection

How long did it take to complete this task?
- took about 8hrs

What went well?
- Was able to understand the read / write lock and how to think about managing them, especially in add_value().

What didn't go well? What would you do differently next time?
- Trying to optimise for the benchmark didn't really go well. I tried various methods and spent the majority of my time here, but initially I was just shooting in the dark. Even after getting more context on the benchmark tests I wasn't really able to improve my timings a lot.


## References

(Any references to other code, documentation, or resources that were helpful in completing this task)
- Mainly the Rust Doc, Ed discussion and primer. Used LLMs for quick examples for Arc/Rwlock and handling type conversions and syntax fixes.