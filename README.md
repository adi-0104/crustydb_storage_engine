# CrustyDB

A Rust-based relational database I built from scratch for the Database Systems course (CMSC 23500) at UChicago. The project was split across three milestones, each adding a new layer to the system.

The base skeleton and course infrastructure were provided by the ChiData group at UChicago. All milestone implementations are my own.

---

## What I worked on

**Milestone 1 - Heap Page**

Implemented the page layout for storing records on disk. A heap page holds fixed-size slots with a slot directory at one end and data growing from the other. Had to handle variable-length records, track free space, compact the page after deletes, and iterate over live tuples. Most of the work was in getting the byte offsets right and making sure the tests passed end to end.

**Milestone 2 - Heapstore and Buffer Pool**

Built the heap file on top of pages - basically the file-backed storage layer for a table. Then implemented the buffer pool, which caches pages in memory to avoid hitting disk every time. The buffer pool uses frames and an eviction policy to decide which pages to flush when the cache is full.

**Milestone 3 - Query Operators**

Implemented the relational operators that actually execute SQL:
- Nested Loop Join
- Hash Join (build hash map on smaller table, probe with larger)
- Grace Hash Join (partitions both tables to disk first, handles cases that don't fit in memory)
- Aggregate with GROUP BY support

These follow the Volcano model - each operator has an `open()`, `next()`, and `close()`, and results flow up the pipeline one tuple at a time.

---

## Running it

You need Rust 1.81+:
```bash
rustup update
```

Start the server:
```bash
cargo run --bin server
```

Connect a client (separate terminal):
```bash
cargo run --bin cli-crusty
```

Basic flow:
```sql
\r testdb
\c testdb
CREATE TABLE test (a INT, b INT, primary key (a));
\i data.csv test
SELECT a, b FROM test;
SELECT sum(a), sum(b) FROM test;
```

Run tests:
```bash
cargo test
cargo test -p heapstore
```

---

## Project structure

```
src/
├── storage/
│   ├── heapstore/     # heap page, heap file, buffer pool
│   └── memstore/      # in-memory reference implementation
├── queryexe/          # query operators (join, aggregate, scan)
├── optimizer/         # query optimizer
├── server/            # server binary
├── cli-crusty/        # psql-style CLI client
└── common/            # shared types, traits, errors
```

---

## What I learnt

- Rust's ownership model made the buffer pool tricky - managing mutable references to frames while also maintaining a hash map over them required some restructuring.
- The Grace Hash Join was the most complex to get right. Getting the partitioning and probing phases to work correctly without double-counting or missing tuples took a few iterations.
- Working in a multi-crate workspace was new to me coming from Python. Understanding how `cargo build -p <crate>` and cross-crate dependencies work took a bit.
- I got a much clearer picture of how a database actually works end to end - before this I mostly thought of it as a black box.

## Possible improvements

- The buffer pool eviction policy is basic. A proper LRU or clock algorithm would be worth implementing.
- Grace Hash Join could be more efficient with better partition sizing to avoid overflow.
- Haven't implemented transactions yet (txn_manager crate is mostly a stub).
