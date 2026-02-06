# Write up

Adithya Suresh

## Design

Apart from implementing the suggested approach for the slotted architecture. I implemented the following design:
- **Heap Header Metadata:** 
    the bytes in the heap metadat track three things total_slots, A free_ptr to track the occupied data bytes bottom-up, and a total_bytes counter for the total data_bytes present at that point in the page. each take about 2 bytes in the Page.
- **Space Reclamation:** 
    I decided to perform the compaction function when we add/update a value in the page based on the contiguous space available in the page. get_free_space() accounts for the dead space in fragmentd regions.
- **Update Value logic:** 
    Data that is of the same length or smaller, is updated in the same space. If the new data length is greater i mark the old space as dead and write it to a new location thats available (if not readily available i perform compaction) 

## Time Estimate / Reflection 

Took about 15-18 hrs over 3 days to complete this. Major time went in trying to understand the assignment and find an optimal way to handle compaction, update value handling for a page. The Serialization logic also took some time to decode but the previous HW helped in some regard.

I was able to strongly understand and implement the page architecture and how rust serialzes/deserializes the data. Still a little bit confused with the implementation of my iter traits.

Somethings i would do better in code:
1. If I had more time, I’d probably refactor these helper functions. Instead of having separate getters and setters for every piece of metadata, I’d use a simple dictionary to track everything in its deserialized form, or at least pair the methods for each property.
2. Use the debugging options and learn how to use the profiler to optimise my code, was only able to use the logging and debugging tools towards the end. 


## References

https://doc.rust-lang.org/std/cmp/trait.PartialOrd.html - mapped it to like implicit dunder methods in python
https://doc.rust-lang.org/rust-by-example/flow_control/if_let.html - If let syntax 
https://doc.rust-lang.org/std/primitive.u32.html#method.saturating_sub - to avoid underflow
https://www.postgresql.org/docs/current/sql-vacuum.html - came across VACUUM when i was looking for strategies to work around compaction.