# Reactive Event Driven Architecture using DAG Graph

## Overview

The Reactive Computation Engine represents a fundamental shift in how distributed databases handle dependencies, events, and computations. By unifying spreadsheet-like reactive computation with event-driven architectures, this system enables Massive Graph to provide instant, zero-copy propagation of changes across complex dependency graphs while supporting everything from microsecond calculations to day-long data pipelines. This same engine handles both computation propagation AND delta propagation to subscribed users.

## Core Innovation

Traditional systems separate reactive computation (spreadsheets), event processing (message queues), and data pipelines (orchestration engines) into distinct architectures. The Reactive Computation Engine unifies these paradigms through a single dependency graph (DAG) where everything - state fields, derived computations, effects, and even user subscriptions - are nodes in the same reactive network.

## Architecture Components

### 1. State Graph Document Type

A new document type that captures the entire reactive dependency network:

```rust
enum DocumentType {
    // ... existing types ...
    StateGraph = 10,  // The reactive dependency graph
}
```

The State Graph:
- Maintains the DAG of all dependencies across the system
- Handles propagation ordering via topological sort
- Controls delta propagation to subscribers
- Ensures correct evaluation order for complex dependency chains
- Deduplicates redundant computations

### 2. Field Types

#### 2.1 State Fields
Any field in the Massive Graph database is implicitly a state field. No explicit declaration is needed - all existing document fields can serve as inputs to derived fields or effects.

#### 2.2 Derived Fields

Computed values that update automatically when their dependencies change:

```rust
enum Value {
    // ... existing types ...
    Derived {
        formula: Formula,
        inputs: Vec<ValueRef>,
        cached: Option<Arc<Value>>,
        last_computed: Timestamp,
    },
}
```

**Key Innovation - Self-Reference**: Derived fields can access their own current value, eliminating the need for separate "stateful" types:
- `NewValue = f(inputs)` - Pure derived field (spreadsheet-like)
- `NewValue = f(inputs, self.current)` - Stateful computation (accumulator-like)

**Output Dimensions**: Derived fields can output any dimensional data:
- **Scalar**: Single values (totals, averages, flags)
- **Column/Vector**: 1D arrays (time series, embeddings)
- **Matrix**: 2D arrays (correlation matrices, attention weights)
- **Tensor**: N-dimensional arrays (ML model weights, feature maps)

Examples:
```rust
// Scalar output
// Document.total = sum(Document.items[*].price)
let total = Derived::sum(Pattern::new("Document.items[*].price"));

// Vector output - moving average window
// Document.movingAvg = slice(Document.values, -30).avg()
let moving_avg = Derived::window_avg("Document.values", 30);

// Matrix output - correlation matrix
// Document.correlations = correlate(Document.features)
let correlations = Derived::correlate("Document.features");

// Self-referencing for running total
// Document.runningTotal = self.current + Document.newValue
let running_total = Derived::accumulate(|self_val, new_val| self_val + new_val);
```

#### 2.3 Effect/Trigger Fields

Fields that don't produce values but cause actions when conditions are met:

```rust
enum EffectField {
    Boolean,    // Simple trigger: false (ready) → true (triggered)
    Counter,    // Multiple pending: increment on trigger, decrement on ack
    Queue,      // Preserve all triggers with timestamps
}
```

Effects bridge the reactive graph to external systems:
- Monitor conditions in the dependency graph
- Fire when conditions are met
- Communicate via state transition deltas
- Can require acknowledgment from external systems

Communication patterns:
- **Fire-and-forget**: No acknowledgment needed
- **Acknowledged**: Wait for confirmation via return delta
- **Counted**: Track multiple pending triggers

Example:
```rust
// Effect that triggers when stream completes
let on_stream_complete = Effect::when(
    |ctx| ctx.get_field("Stream.complete") == Value::Bool(true)
);

// Backend subscribes to this effect field
// When it changes false→true, backend executes action
// Backend sends acknowledgment delta: true→false
```

#### 2.4 Optimizable Fields (Future Capability)

Variables that can be adjusted during optimization passes:
```rust
let price = Optimizable::new(10.0, 100.0);  // min: 10, max: 100
let weights = Optimizable::tensor([784, 10], -1.0, 1.0);  // shape with bounds
```

#### 2.5 Goal Fields (Future Capability)

Define optimization objectives that drive backward passes through the DAG:
```rust
let max_profit = Goal::maximize(|ctx| {
    ctx.get("Revenue") - ctx.get("Costs")
});

let min_error = Goal::minimize(|predictions, labels| {
    mse(predictions, labels)
});

let target_temp = Goal::target("Temperature", 22.5);
```

### 3. Execution Timing

Controls when derived fields and effects execute. This is simplified pending further design:

- **Immediate**: Execute as soon as dependencies change (spreadsheet-like)
- **On Completion**: When a stream or operation completes
- **Conditional**: When specific predicates are met
- **Temporal**: Based on time/schedule
- **Batched**: Accumulate changes for resource efficiency

### 4. Propagation Mechanisms

#### 4.1 Forward Pass (Traditional)
Standard reactive propagation through the DAG:
```
State Change → Derived Update → Effect Trigger → External Action
```

#### 4.2 Backward Pass (Optimization - Future)
Gradient/adjustment flow for ML and optimization:
```
Goal ← Gradients ← Optimizable Fields
```

This enables first-class ML model training within the database:
- Model weights as optimizable fields
- Loss functions as goal fields
- Training happens through DAG propagation
- Automatic differentiation for gradient computation
- Version history provides checkpoint/rollback capability

### 5. Communication via Deltas

#### 5.1 Unified Delta System

All changes communicate through the same delta mechanism:
- State field updates generate deltas
- Derived field recomputations generate deltas
- Effect state transitions generate deltas
- All deltas propagate through the same infrastructure

#### 5.2 Effect Communication Pattern

Effects communicate with external systems through state transition deltas:

```
1. Condition met in DAG
2. Effect field: false → true (Delta #1)
3. Delta propagates to subscribed backend
4. Backend processes action
5. Backend acknowledges: true → false (Delta #2)
6. Effect ready for next trigger
```

For high-throughput scenarios, versioning ensures no triggers are lost - each state transition creates a new version that backends can process independently.

### 6. Versioning

Field versioning is optional for state and derived fields but **required** for effects:

- **State/Derived Fields**: Optional versioning for audit trails
- **Effect Fields**: Mandatory versioning to track:
  - When triggered (version N: false→true)
  - When acknowledged (version N+1: true→false)
  - Complete history of all triggers
  - Timing information for performance analysis

The version history provides natural audit trails and enables replay of effect sequences.

### 7. Machine Learning as First-Class Citizen

The reactive architecture naturally supports ML workflows:

#### Training Loops
```rust
// Model weights as optimizable fields
let weights = Optimizable::xavier_init([784, 10]);

// Forward pass as derived fields
let predictions = Derived::compute(|ctx| {
    model(ctx.get("Inputs"), ctx.get("Weights"))
});

let loss = Derived::compute(|ctx| {
    cross_entropy(ctx.get("Predictions"), ctx.get("Labels"))
});

// Backward pass through goal field
let training = Goal::minimize(loss)
    .optimizer(Optimizer::AdamW { lr: 0.001 })
    .batch_size(32);
```

#### Automatic Differentiation
- Derived fields can compute gradients
- Gradients propagate backward through DAG
- Weights update based on optimization policy

#### Experiment Tracking
- Every training step versioned automatically
- Model checkpoints are just field versions
- Rollback to any previous state
- A/B testing through parallel DAG branches

### 8. Unification Principles

#### Everything is a Node
- All fields are potential nodes in the DAG
- Derived fields and effects declare dependencies explicitly
- User subscriptions are edges in the graph
- Single propagation engine handles all updates

#### Subscription as Computation
Delta propagation to users is just another effect in the DAG:
```rust
// User subscription is an effect
let user_notification = Effect::when_changed("Document")
    .action(|delta| send_delta_to_user(user_id, delta));
```
No special subscription mechanism needed - it's just another node in the reactive graph.

#### Unified Timescales
The same architecture handles:
- Microsecond spreadsheet calculations
- Second-scale stream processing
- Minute-scale API orchestration
- Hour-scale batch processing
- Day-scale data pipelines

All using the same DAG propagation with different execution timing policies.

## Implementation Considerations

### Cycle Detection
The State Graph must detect and handle circular dependencies:
- Immediate cycles rejected at definition time
- Temporal cycles allowed (self-reference to previous values)
- Warning system for potential indirect cycles

### Performance Optimizations
- Incremental computation where possible
- Parallel evaluation of independent branches
- Lazy evaluation for expensive computations
- Caching strategies for frequently accessed derived values

### Failure Handling
- Effects track acknowledgment timeouts
- Failed computations don't propagate invalid values
- Retry policies configurable per field
- Dead letter queues for persistent failures

## Summary

The Reactive Computation Engine unifies traditionally separate systems - spreadsheets, event processing, data pipelines, and ML training - into a single, elegant architecture. By treating everything as nodes in a reactive DAG and using deltas as the universal communication mechanism, Massive Graph provides a powerful, efficient, and intuitive platform for building reactive, intelligent applications at any scale.