# CrustyDB — Relational Database Engine in Rust

A functional relational database built from scratch in Rust, developed as part of the **Database Systems** course (CMSC 23500) at the University of Chicago.

This isn't just a toy — CrustyDB runs real SQL queries. You can start a server, connect a client, create tables, import data, and run `SELECT`, `JOIN`, and `GROUP BY` queries end-to-end. Everything from the raw bytes on disk to the query result in your terminal was implemented from the ground up.

---

## What I Built

Most databases are black boxes — you type SQL and results appear. In this project, I built the internals layer by layer:

### 1. Storage Engine — Pages & Heap Files
At the lowest level, a database is just bytes on disk organized into **pages** (fixed-size chunks, like 4KB blocks). I implemented:
- A **heap page** format with a slot directory that tracks which records live where, handles variable-length records, compacts free space when records are deleted, and iterates efficiently over stored tuples
- A **heap file** built on top of pages — the actual file on disk where a table's data lives, supporting insert, delete, update, and full-table scans

### 2. Buffer Pool
Reading from disk is slow. Databases use a **buffer pool** — an in-memory cache of recently used pages. I implemented a buffer pool with:
- A frame-based cache that holds hot pages in memory
- An eviction policy to decide which pages to flush to disk when the cache is full
- Statistics tracking for cache hit/miss rates

### 3. Query Execution Engine
Once data can be stored and retrieved, the database needs to *do something* with it. I implemented a set of **relational operators** that form a pipeline to execute SQL queries:

| Operator | What it does |
|---|---|
| **Nested Loop Join** | Combines two tables by checking every pair of rows — simple but correct |
| **Hash Join** | Faster join using a hash table — build a hash map on the smaller table, probe with the larger |
| **Grace Hash Join** | Handles joins too large to fit in memory by partitioning both tables to disk first |
| **Aggregate** | Computes `SUM`, `COUNT`, `AVG`, etc., with support for `GROUP BY` |

These operators follow the **Volcano model** — each operator exposes a `next()` method, and results flow up the pipeline one tuple at a time, just like a real database.

---

## Tech Stack

- **Language:** Rust
- **Architecture:** Multi-crate Cargo workspace
- **Storage:** Custom heap file format (no external libraries)
- **Query execution:** Volcano-style iterator model
- **Interface:** SQL via a CLI client (psql-style commands)

---

## Project Structure

```
src/
├── storage/
│   ├── heapstore/     # Heap page, heap file, buffer pool
│   └── memstore/      # In-memory storage (reference implementation)
├── queryexe/          # Query operators: join, aggregate, scan
├── optimizer/         # Query optimizer
├── server/            # Server binary — connects everything together
├── cli-crusty/        # Command-line client (like psql)
├── common/            # Shared types, traits, error definitions
└── txn_manager/       # Transaction manager
```

---

## Running It

You need Rust 1.81+. Install or update via:
```bash
rustup update
```

**Start the server:**
```bash
cargo run --bin server
```

**Connect a client (in a separate terminal):**
```bash
cargo run --bin cli-crusty
```

**Try it out:**
```sql
-- Create a database and connect
\r mydb
\c mydb

-- Create a table
CREATE TABLE employees (id INT, name TEXT, salary INT, primary key (id));

-- Import data from a CSV
\i data.csv employees

-- Run queries
SELECT name, salary FROM employees;
SELECT SUM(salary) FROM employees;
```

**Run the test suite:**
```bash
cargo test
```

---

## Course Context

This was built as part of **MPCS / CMSC 23500 — Introduction to Database Systems** at the University of Chicago (Winter 2026). The project was structured as a series of milestones, each unlocking a new layer of the database:

1. **Milestone 1 — Heap Page:** Byte-level page layout, slot directory, compaction
2. **Milestone 2 — Heapstore & Buffer Pool:** File-backed storage, in-memory page cache
3. **Milestone 3 — Query Operators:** Joins (NLJ, Hash, Grace Hash) and aggregates

The base skeleton and course infrastructure were provided by the [ChiData group](https://cs.uchicago.edu/research/groups/chidata/) at UChicago. All milestone implementations are my own work.
