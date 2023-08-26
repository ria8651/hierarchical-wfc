# API Design

## Interfaces

`Tileset` and `WFC Backend` are the WFC concepts while `Workflow` and `HWFC Backend` are our extensions. Ideally you would be able to mix and match `Tileset`'s in a `Workflow`, but this will make the implementation much harder. Instead, `Workflow`'s will be self-contained.

### Tileset

Should contain information on:

- Graph structure
- Constraints

### WFC Backend

Takes in an instance of a tileset and collapses it.

### Workflow

Contains information on:

- Graph transformations
- Tilesets to use at each step

### HWFC Backend

Takes in an instance of a `Workflow` and collapses it.

## API Examples

`Workflow` creation:

```rust
let castle_workflow = Workflow::new(semantics, vec![
    Pass::Wfc(building_pass),
    Pass::Transformation(facade_divide),
    Pass::Wfc(facade_pass),
]);
```

`Workflow` running:

```rust
let result = CpuRunner::run(castle_workflow, graph);
```
