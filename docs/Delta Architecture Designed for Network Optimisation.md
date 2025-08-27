# Delta Architecture Designed for Network Optimisation

### Core Design Philosophy
The delta architecture is designed around immutability and zero-copy principles to achieve maximum network throughput. Traditional database replication systems often require serialisation, deserialisation, and multiple memory copies as data moves from storage through application layers to the network. Our approach treats deltas as immutable from creation, allowing direct memory mapping from storage to network buffers without intermediate copies. This design choice fundamentally shapes the entire system - once a delta is created, its bytes never change, enabling safe concurrent access without synchronisation.

The choice of a binary format over text-based formats like JSON was deliberate. Binary encoding provides predictable field offsets, enabling direct field access without parsing. Fixed-size headers allow routers and middleware to inspect and route deltas without deserialising payloads. The compact representation minimises network bandwidth, critical when synchronising millions of deltas per second across distributed nodes.

### Core Principles
- All deltas are immutable to facilitate efficient zero copy propagation to the network
- When received, deltas are wrapped in a server-generated header, which is also immutable
- Two header types: minimal (32 bytes) for trusted networks, secure (64 bytes) for zero-trust environments
- Cryptographic lineage ensures strict ordering and integrity through chained MACs (secure mode only)

### Binary Layout

#### Delta Structure
- **Target Document ID**: 16 bytes (can be represented as base62 string)
- **Operation Type**: 1 byte (enum, max 256 operations)
- **Pattern ID**: 1-3 bytes (using variable-length encoding)
- **Parameter Length**: 2 bytes (for pattern parameters if needed)
- **Parameters**: Variable length (array indices, map keys, etc.)
- **Payload Length**: 4 bytes (supports up to 4GB payloads)
- **Payload**: Variable length

Total delta overhead: 24+ bytes depending on pattern parameters

#### Operation Categories
- **Property-level operations**: append, replace, add (applied to specific properties or patterns)
- **Document-level operations**: create, move, create field
- **Meta operations**: operations on meta properties
- **Delta group**: special operation type containing multiple deltas as payload

#### Standard Header (32 bytes fixed) - For Trusted Environments
- **Delta ID**: 8 bytes (unique identifier, can be viewed as base62 string)
- **Timestamp**: 8 bytes (Unix timestamp with microseconds, used for ordering)
- **Checksum**: 8 bytes (CRC64-Jones for high speed error detection)
- **Flags**: 1 byte (header type, compression, etc.)
- **Reserved**: 7 bytes (padding to reach 32 bytes, available for future use)

Total wire format: 32-byte header + delta(s)

#### Secure Header (64 bytes fixed) - For Zero-Trust Environments
- **Delta ID**: 8 bytes (unique identifier, can be viewed as base62 string)
- **Previous Delta ID**: 8 bytes (ensures lineage/sequence, 0x0 for first delta)
- **Timestamp Received**: 8 bytes (Unix timestamp with microseconds)
- **Processing Duration**: 2 bytes (milliseconds between received and applied)
- **Flags**: 1 byte (header type, compression, etc.)
- **Reserved**: 1 byte (padding/future use)
- **BLAKE3 MAC**: 36 bytes (authentication and integrity)

Total wire format: 64-byte header + delta(s)

#### Flags Byte Configuration
- **Bit 0**: Header type (0=standard 32-byte, 1=secure 64-byte)
- **Bits 1-2**: Compression type (00=none, 01=lz4, 10=zstd, 11=reserved)
- **Bits 3-7**: Reserved for future use

The dual-header design recognises that not all environments require cryptographic security. Within trusted networks or local processing pipelines, the lighter 32-byte header reduces overhead by 50% whilst maintaining error detection through CRC32. The secure header is used for untrusted networks, providing cryptographic proof of lineage and tamper detection. The flags byte allows receivers to identify header type from the first byte, enabling efficient routing decisions.

#### Wire Format Diagram
```
Standard Header (32 bytes):
┌─────────────────────────────────────────────────────────────┐
│  Delta ID     │  Timestamp    │ CRC32 │Flags│   Reserved    │
│  (8 bytes)    │  (8 bytes)    │ (4B)  │(1B) │   (11 bytes)  │
└───────────────┴───────────────┴───────┴─────┴───────────────┘

Secure Header (64 bytes):
┌─────────────────────────────────────────────────────────────┐
│                    Server Header (64 bytes)                  │
├───────────────────┬───────────────────┬────────────────────┐
│  Delta ID         │  Previous Delta   │  Timestamp         │
│  (8 bytes)        │  ID (8 bytes)     │  Received (8 bytes)│
├───────────────────┴───────────────────┴────────────────────┤
│  Processing  │ Flags │ Reserved │                           │
│  Duration    │ (1B)  │  (1B)    │     BLAKE3 MAC           │
│  (2 bytes)   │       │          │     (36 bytes)           │
└──────────────┴───────┴──────────┴───────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                      Delta Payload                           │
├───────────────────┬──────────┬─────────────┬───────────────┤
│  Target Doc ID    │ Operation│ Payload Len │    Payload    │
│  (16 bytes)       │ Type (1B)│  (4 bytes)  │  (variable)   │
└───────────────────┴──────────┴─────────────┴───────────────┘
```

#### Cryptographic Lineage Verification (Secure Mode Only)

```
Delta N-1                    Delta N                    Delta N+1
┌─────────┐                ┌─────────┐                ┌─────────┐
│ ID: 123 │◄───────────────│ ID: 456 │◄───────────────│ ID: 789 │
│ Prev: 0 │                │ Prev:123│                │ Prev:456│
└─────────┘                └─────────┘                └─────────┘
     │                          │                          │
     └──────────┬───────────────┴──────────────────────────┘
                │
                ▼
        ┌──────────────────────────────────┐
        │  MAC Computation for Delta N:    │
        │                                   │
        │  Input = PrevID || CurrentID ||  │
        │         Timestamp || Payload     │
        │                                   │
        │  MAC = BLAKE3_MAC(Key, Input)    │
        └──────────────────────────────────┘
                │
                ▼
        ┌──────────────────────────────────┐
        │  Verification Process:           │
        │  1. Check PrevID matches N-1     │
        │  2. Compute expected MAC         │
        │  3. Compare with stored MAC      │
        │  ✓ Both must pass               │
        └──────────────────────────────────┘
```

### Security Properties
- **Lineage Integrity**: Cannot insert, delete, or reorder deltas without detection (secure mode)
- **Performance**: BLAKE3 processes multiple GB/s, supporting millions of deltas/second
- **Chain Security**: Compromise of one delta requires recomputing all subsequent MACs
- **Zero-Copy Ready**: Immutable structure allows direct memory mapping and network transmission
- **Flexible Security**: Choice of header based on trust model and performance requirements

### Delta Groups
- Represented as a special operation type with multiple deltas as payload
- Maintains position in main lineage chain via Previous Delta ID (secure mode) or timestamp ordering (standard mode)
- Internal deltas chain together without full server headers
- Atomic execution: all deltas in group succeed or fail together
- Internal delta format: simplified header with just Previous Delta ID within group

The delta group design enables atomic multi-operation transactions whilst maintaining the efficiency of the delta architecture. By treating the group as a single delta in the main chain, we preserve lineage guarantees whilst allowing the internal deltas to use minimal overhead. This is particularly valuable for complex operations that must succeed or fail as a unit, such as moving a node with all its edges in a graph structure.

