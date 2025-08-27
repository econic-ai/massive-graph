# Delta Operations Specification

## Overview

Delta operations are the fundamental units of change in Massive Graph. Each operation represents an atomic modification to a document, designed for efficient network transmission, immutable storage, and concurrent application. Operations are categorised by their scope and applicability to different document types, with each operation containing the minimal information necessary to transform a document from one valid state to another.

This specification defines all available delta operations, their parameters, and their applicability across the ten document types supported by Massive Graph. Operations are designed to be composable, idempotent where possible, and efficient for both application and network transmission.

## Operation Categories

### Property Operations

Property operations modify individual values within documents. These operations use the pattern system to address specific locations within any document structure, whether a simple key-value pair, an array element, or a tensor slice.

#### Set
**Description**: Sets a property to a specific value, creating it if it doesn't exist or replacing the current value entirely.  
**Parameters**: Pattern ID, optional parameters (indices/keys), value  
**Applies to**: Json, Graph, Table, TimeSeries, Geospatial  
**Example**: Setting user.name to "Alice", or tensor[5,10] to 3.14

#### Update
**Description**: Modifies an existing property value in place, failing if the property doesn't exist. Ensures the property was previously initialised.  
**Parameters**: Pattern ID, optional parameters, new value  
**Applies to**: Json, Graph, Table, TimeSeries, Geospatial  
**Note**: Stricter than Set - requires property existence

#### Delete
**Description**: Removes a property entirely from the document. The property path is eliminated, not just set to null.  
**Parameters**: Pattern ID, optional parameters  
**Applies to**: Json, Graph, Table, TimeSeries, Geospatial  
**Note**: In tables, may trigger column removal if last value

#### Increment
**Description**: Atomically increments a numeric property by a specified amount. Useful for counters and statistics.  
**Parameters**: Pattern ID, optional parameters, increment value (can be negative)  
**Applies to**: Json, Graph, Table, TimeSeries, Tensor  
**Constraint**: Property must be numeric type

### Collection Operations

Collection operations handle multiple values as a unit, whether arrays, lists, or other collection types. These operations maintain collection integrity whilst enabling efficient modifications.

#### Append
**Description**: Adds one or more elements to the end of a collection. For arrays and lists, maintains order. For streams, adds new chunks.  
**Parameters**: Pattern ID, values to append  
**Applies to**: Json (arrays), Stream, Graph (edge lists), Table (rows), TimeSeries (data points), Event  
**Note**: O(1) operation for streams and properly structured arrays

#### Prepend
**Description**: Adds one or more elements to the beginning of a collection. May require reindexing for some structures.  
**Parameters**: Pattern ID, values to prepend  
**Applies to**: Json (arrays), Table (rows), TimeSeries  
**Performance**: May be O(n) for some structures

#### InsertAt
**Description**: Inserts elements at a specific index within an ordered collection. Shifts subsequent elements.  
**Parameters**: Pattern ID, index, values to insert  
**Applies to**: Json (arrays), Table (rows), TimeSeries  
**Note**: Triggers index updates for subsequent elements

#### RemoveAt
**Description**: Removes elements at specific indices from a collection. Shifts subsequent elements to fill gaps.  
**Parameters**: Pattern ID, start index, count  
**Applies to**: Json (arrays), Table (rows), TimeSeries  
**Note**: Maintains collection continuity

#### RemoveWhere
**Description**: Removes all elements matching a predicate condition. Useful for bulk filtering operations.  
**Parameters**: Pattern ID, predicate specification  
**Applies to**: Json (arrays), Graph (conditional edge removal), Table (row filtering)  
**Note**: Atomic operation despite potentially affecting multiple elements

#### Clear
**Description**: Removes all elements from a collection whilst preserving the collection structure itself.  
**Parameters**: Pattern ID  
**Applies to**: Json (arrays/maps), Graph (clear all edges), Table (clear all rows)  
**Note**: Collection remains but becomes empty

### Stream Operations

Stream operations are specialised for append-only sequences. These operations are optimised for sequential data like logs, media streams, and audit trails. Streams are immutable once written - chunks can only be added, never modified or removed.

#### StreamAppend
**Description**: Appends a new chunk to a stream. Chunks are immutable once written.  
**Parameters**: Stream ID, chunk data, optional metadata  
**Applies to**: Stream, Event  
**Note**: Creates new delta in storage, updates stream linked list

#### StreamMark
**Description**: Places a named marker at the current stream position for later reference.  
**Parameters**: Stream ID, marker name, optional metadata  
**Applies to**: Stream, Event  
**Use case**: Checkpoints, chapter markers in media

#### StreamClose
**Description**: Marks a stream as complete, preventing further appends.  
**Parameters**: Stream ID, optional final metadata  
**Applies to**: Stream, Event  
**Note**: Administrative operation, stream remains readable

### Text Operations

Text operations provide specialised support for collaborative text editing, maintaining position stability and enabling efficient concurrent modifications.

#### TextInsert
**Description**: Inserts text at a specific position or after an anchor point. Supports both index-based and anchor-based positioning.  
**Parameters**: Position/anchor, text to insert, optional anchor metadata  
**Applies to**: TextFile, Json (string properties)  
**Note**: Updates piece table, maintains anchors

#### TextDelete
**Description**: Removes text between two positions or relative to an anchor.  
**Parameters**: Start position/anchor, end position/length  
**Applies to**: TextFile, Json (string properties)  
**Note**: Adjusts piece boundaries without moving data

#### TextReplace
**Description**: Atomically replaces text between two positions with new text. Equivalent to delete + insert but atomic.  
**Parameters**: Start position, end position, replacement text  
**Applies to**: TextFile, Json (string properties)  
**Use case**: Find and replace operations

#### LineInsert
**Description**: Inserts one or more complete lines at a specific line number or anchor.  
**Parameters**: Line number/anchor, lines to insert  
**Applies to**: TextFile  
**Note**: Maintains line ID stability for collaboration

#### LineDelete
**Description**: Removes one or more complete lines.  
**Parameters**: Start line, line count  
**Applies to**: TextFile  
**Note**: Updates line index without affecting other line IDs

#### LineUpdate
**Description**: Replaces the content of a specific line whilst maintaining its line ID.  
**Parameters**: Line ID, new content  
**Applies to**: TextFile  
**Note**: Preserves collaborative anchors

### Binary Operations

Binary operations handle raw byte sequences and large binary objects efficiently, supporting streaming and partial updates.

#### BinaryWrite
**Description**: Writes bytes at a specific offset, potentially extending the binary data.  
**Parameters**: Offset, byte data  
**Applies to**: Binary  
**Note**: May extend file if offset + length > current size

#### BinaryAppend
**Description**: Appends bytes to the end of binary data.  
**Parameters**: Byte data  
**Applies to**: Binary  
**Note**: Optimised for streaming scenarios

#### BinaryTruncate
**Description**: Truncates binary data to a specified length.  
**Parameters**: New length  
**Applies to**: Binary  
**Use case**: File size reduction

#### BinaryPatch
**Description**: Applies a binary diff patch to update portions of binary data efficiently.  
**Parameters**: Patch specification (diff format)  
**Applies to**: Binary  
**Use case**: Efficient large file updates

### Graph Operations

Graph operations maintain graph structure integrity whilst enabling efficient traversal and modification of nodes and edges.

#### NodeCreate
**Description**: Creates a new node with properties in the graph.  
**Parameters**: Node ID, node type, properties  
**Applies to**: Graph  
**Note**: Node becomes available for edge connections

#### NodeUpdate
**Description**: Updates properties of an existing node.  
**Parameters**: Node ID, property updates  
**Applies to**: Graph  
**Note**: Preserves node identity and edges

#### NodeDelete
**Description**: Removes a node and optionally its connected edges.  
**Parameters**: Node ID, cascade option  
**Applies to**: Graph  
**Note**: Can trigger cascade deletion of edges

#### EdgeCreate
**Description**: Creates a directed or undirected edge between two nodes.  
**Parameters**: Source node ID, target node ID, edge type, properties  
**Applies to**: Graph  
**Constraint**: Both nodes must exist

#### EdgeUpdate
**Description**: Updates properties of an existing edge.  
**Parameters**: Edge ID, property updates  
**Applies to**: Graph  
**Note**: Maintains connectivity

#### EdgeDelete
**Description**: Removes an edge from the graph.  
**Parameters**: Edge ID or source-target pair  
**Applies to**: Graph  
**Note**: Nodes remain unaffected

#### SubgraphMerge
**Description**: Merges another graph structure into this graph, handling ID conflicts.  
**Parameters**: Subgraph data, conflict resolution strategy  
**Applies to**: Graph  
**Use case**: Combining graph fragments

### Tensor Operations

Tensor operations support efficient manipulation of multi-dimensional arrays for machine learning and scientific computing workloads.

#### TensorReshape
**Description**: Changes tensor dimensions without modifying underlying data order.  
**Parameters**: New shape specification  
**Applies to**: Tensor  
**Constraint**: Total elements must remain constant

#### TensorSliceUpdate
**Description**: Updates a slice of the tensor with new values.  
**Parameters**: Slice specification (start/end per dimension), values  
**Applies to**: Tensor  
**Note**: Creates new piece in piece table

#### TensorTranspose
**Description**: Reorders tensor dimensions.  
**Parameters**: Dimension permutation  
**Applies to**: Tensor  
**Note**: Metadata operation, no data movement

#### TensorConcat
**Description**: Concatenates another tensor along a specified dimension.  
**Parameters**: Dimension, tensor data  
**Applies to**: Tensor  
**Constraint**: Other dimensions must match

#### TensorSqueeze
**Description**: Removes dimensions of size 1 from the tensor shape.  
**Parameters**: Optional dimension indices  
**Applies to**: Tensor  
**Use case**: Dimension reduction

#### TensorUnsqueeze
**Description**: Adds dimensions of size 1 at specified positions.  
**Parameters**: Dimension indices  
**Applies to**: Tensor  
**Use case**: Broadcasting preparation

### Table Operations

Table operations provide columnar data manipulation with schema awareness and efficient row/column operations.

#### ColumnAdd
**Description**: Adds a new column to the table schema.  
**Parameters**: Column name, data type, optional default value  
**Applies to**: Table  
**Note**: Retroactively applies to all existing rows

#### ColumnRemove
**Description**: Removes a column and all its data.  
**Parameters**: Column name  
**Applies to**: Table  
**Note**: Permanent data removal

#### ColumnRename
**Description**: Changes the name of a column whilst preserving data.  
**Parameters**: Old name, new name  
**Applies to**: Table  
**Note**: Updates schema only

#### RowInsert
**Description**: Inserts one or more rows with specified column values.  
**Parameters**: Row data (column-value pairs)  
**Applies to**: Table  
**Note**: Missing columns use defaults or null

#### RowUpdate
**Description**: Updates specific columns in rows matching criteria.  
**Parameters**: Row selector, column updates  
**Applies to**: Table  
**Note**: Atomic across all matching rows

#### RowDelete
**Description**: Removes rows matching specified criteria.  
**Parameters**: Row selector/predicate  
**Applies to**: Table  
**Use case**: Data cleanup, filtering

### Time Series Operations

Time series operations maintain temporal ordering and enable efficient time-based queries and aggregations.

#### TimePointAdd
**Description**: Adds a data point at a specific timestamp.  
**Parameters**: Timestamp, value(s), optional tags  
**Applies to**: TimeSeries  
**Constraint**: May reject if timestamp ordering violated

#### TimeRangeDelete
**Description**: Removes all data points within a time range.  
**Parameters**: Start timestamp, end timestamp  
**Applies to**: TimeSeries  
**Use case**: Data retention policies

#### TimeSeriesResample
**Description**: Changes the sampling frequency of time series data.  
**Parameters**: New frequency, aggregation method  
**Applies to**: TimeSeries  
**Note**: May involve interpolation or aggregation

#### TimeSeriesAggregate
**Description**: Computes aggregations over time windows.  
**Parameters**: Window size, aggregation function  
**Applies to**: TimeSeries  
**Output**: New aggregated series or values

### Geospatial Operations

Geospatial operations handle geographic data with spatial indexing and geometric transformations.

#### GeometrySet
**Description**: Sets or updates a geometric shape (point, line, polygon).  
**Parameters**: Geometry specification (WKT/WKB format)  
**Applies to**: Geospatial  
**Note**: Updates spatial indices

#### GeometryTransform
**Description**: Applies spatial transformation (translate, rotate, scale).  
**Parameters**: Transformation matrix or parameters  
**Applies to**: Geospatial  
**Note**: Preserves topology

#### SpatialIndexUpdate
**Description**: Rebuilds or updates spatial indices for efficient queries.  
**Parameters**: Index type, parameters  
**Applies to**: Geospatial  
**Use case**: Query optimisation

### Document Operations

Document operations affect the entire document structure or metadata, rather than specific properties.

#### DocumentCreate
**Description**: Creates a new document with initial content.  
**Parameters**: Document ID, document type, initial value  
**Applies to**: All types  
**Note**: Establishes document in system

#### DocumentReplace
**Description**: Completely replaces document content whilst maintaining identity.  
**Parameters**: New content  
**Applies to**: All types  
**Warning**: Destructive operation

#### DocumentMerge
**Description**: Merges another document's content into this document.  
**Parameters**: Source document, merge strategy  
**Applies to**: Json, Graph, Table  
**Note**: Handles conflicts per strategy

#### DocumentClear
**Description**: Removes all content but preserves document structure and metadata.  
**Parameters**: None  
**Applies to**: All types  
**Note**: Document remains but empty

### Meta Operations

Meta operations modify document metadata, indices, and auxiliary structures without changing primary content.

#### MetaSet
**Description**: Sets a metadata property on the document.  
**Parameters**: Meta key, value  
**Applies to**: All types  
**Example**: Setting permissions, tags, or timestamps

#### IndexCreate
**Description**: Creates a new index on specified properties for query optimisation.  
**Parameters**: Index specification, properties  
**Applies to**: Json, Graph, Table, TimeSeries, Geospatial  
**Note**: Background operation

#### IndexDrop
**Description**: Removes an index to save space or change query patterns.  
**Parameters**: Index name  
**Applies to**: Json, Graph, Table, TimeSeries, Geospatial  
**Note**: May affect query performance

#### AnchorCreate
**Description**: Creates a stable anchor point for collaborative editing.  
**Parameters**: Anchor ID, position, anchor type  
**Applies to**: TextFile, Json  
**Use case**: AI agents, collaborative cursors

#### AnchorUpdate
**Description**: Updates an anchor's metadata or position.  
**Parameters**: Anchor ID, updates  
**Applies to**: TextFile, Json  
**Note**: Maintains anchor stability

#### AnchorDelete
**Description**: Removes an anchor point.  
**Parameters**: Anchor ID  
**Applies to**: TextFile, Json  
**Note**: May affect dependent operations

### Transaction Operations

Transaction operations enable atomic multi-operation changes, essential for maintaining consistency in complex updates.

#### TransactionBegin
**Description**: Starts a transaction boundary for atomic operations.  
**Parameters**: Transaction ID, isolation level  
**Applies to**: All types  
**Note**: Subsequent operations are buffered

#### TransactionCommit
**Description**: Atomically applies all operations in the transaction.  
**Parameters**: Transaction ID  
**Applies to**: All types  
**Note**: All succeed or all fail

#### TransactionRollback
**Description**: Cancels a transaction and discards all pending operations.  
**Parameters**: Transaction ID  
**Applies to**: All types  
**Use case**: Error recovery

#### DeltaGroup
**Description**: Bundles multiple deltas for atomic application.  
**Parameters**: Array of deltas  
**Applies to**: All types  
**Note**: More lightweight than full transactions

## Operation Properties

### Idempotency

Certain operations are designed to be idempotent, meaning repeated application produces the same result:
- Set (always results in the same final value)
- DocumentReplace (always results in the same document)
- Clear operations (always results in empty state)

### Commutativity

Some operations can be applied in any order with the same result:
- Increment operations on different properties
- Append operations on different streams
- Node/edge creation with different IDs

### Conflict Resolution

When operations conflict, resolution follows these principles:
1. **Last-writer-wins** for Set operations
2. **Additive** for Increment operations
3. **Preserve-both** for Append operations
4. **First-writer-wins** for Create operations with same ID

## Implementation Notes

Each operation must:
1. Serialize to the binary delta format efficiently
2. Include sufficient information for replay/undo
3. Validate parameters before application
4. Update relevant indices and metadata
5. Trigger appropriate change notifications
6. Maintain document consistency invariants

Operations are designed to be:
- **Minimal**: Contain only necessary information
- **Atomic**: Complete fully or not at all
- **Traceable**: Include metadata for audit trails
- **Efficient**: Optimised for common patterns
- **Composable**: Can be combined into complex operations