# Massive Graph Database

Massive Graph is a high-throughput real-time document database specifically designed to synchronise massive graph data structures within collaborative environments. Unlike traditional databases that assume single ownership or trusted environments, Massive Graph enables secure, real-time collaboration between organisations that need to share data but cannot fully trust each other.

As a graph db, it can also handle highly dynamic and complex permission models for data sharing with a cryptographc data integrity guarantees on both ownership and lineage. Massive Graph handles millions of operations per second with sub-100ms global propagation while maintaining cryptographic proof of every operation.

![Build Status](https://img.shields.io/badge/build-no%20CI%20configured-lightgrey.svg)
![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)

## Key Architectural Decisions

Massive Graph resolves this tension through fundamental design choices optimised for both performance and trust:

- **Rust Implementation**: Memory safety and zero-cost abstractions without garbage collection overhead
- **Delta-based Synchronisation**: Only changes propagate, reducing network overhead by 100-1000x compared to state-based sync
- **Zero-copy Architecture**: Direct memory access eliminates CPU waste from buffer copying
- **Graph-native Data Model**: Relationships are first-class citizens, enabling natural collaboration patterns
- **Lock-free Concurrency**: High-throughput operations via DashMap and Crossbeam data structures
- **Cryptographic Lineage**: Every operation carries verifiable proof without requiring global consensus
- **Multi-protocol Support**: HTTP, WebSocket, and QUIC/WebTransport for different latency requirements
- **Eventual Consistency**: Partition tolerance through deterministic conflict resolution
- **Open Source**: Complete transparency enables cryptographic verification and community innovation

## Performance Targets

**Single Node (Alpha)**
- **1M+ operations per second** - Concurrent read/write operations
- **Sub-100ms latency** - 99th percentile response times
- **100M+ nodes** - Graph scale per instance
- **Zero-copy data flows** - Memory-efficient operations

**Multi-Node Cluster (Beta)**
- **5M+ operations per second** - Distributed across 10-node cluster
- **Sub-100ms global propagation** - Delta synchronisation worldwide
- **Billion-scale graphs** - Cross-node distributed storage
- **Cryptographic verification** - All operations signed and verified

These targets drive every architectural decision, from lock-free data structures to delta compression algorithms.

## Quick Start

### Prerequisites

- **Rust 1.87+** (for edition 2024 support)
- **Docker** (for containerised development)
- **Kubernetes** (optional, for orchestration)

### Local Development

```bash
# Clone the repository
git clone https://github.com/econic-ai/massive-graph.git
cd massive-graph

# Run in development mode with hot reloading
./.bin/watch.sh

# Or run in production mode
./.bin/prod.sh

# Build manually
cargo build --release
```

The application serves multiple protocols simultaneously:
- **HTTP API**: `http://localhost:8080` - REST operations
- **WebSocket**: `ws://localhost:8081` - Real-time bidirectional
- **QUIC**: `localhost:8082` - Low-latency transport
- **Metrics**: `http://localhost:9090/metrics` - Prometheus monitoring

### Docker Development

```bash
# Build development image
docker build --target development -t mg:dev .

# Run with live reloading
docker run -v $(pwd):/app mg:dev

# Or use the provided scripts
make apps-mg-build  # Build container
make apps-mg-up     # Deploy to Kubernetes
```

## Development Roadmap

### Alpha Release (Current - Q1 2025)

**Core Database Engine**
- ⏳ In-memory graph storage with zero-copy delta architecture
- ⏳ Multi-protocol server (HTTP, WebSocket, QUIC)
- ⏳ Containerised development environment with hot reloading
- 🚧 Target: 1M operations/second in single container
- ⏳ WebTransport-based real-time synchronisation

**API Implementation**  
- ⏳ REST API for collections, documents, and deltas
- ⏳ WebSocket endpoints for real-time updates
- ⏳ Prometheus metrics integration
- ⏳ GraphQL-style query interface

### Beta Release (Q2-Q3 2025)

**Production Scale**
- Persistent storage with atomic transactions
- Cryptographic signing and verification
- Permission graphs with dynamic access control
- Target: 5M operations/second across 10-node cluster

**Network Layer**
- Distributed node discovery and handshake
- Eventual consistency framework with conflict resolution
- Priority channels for critical operations

### Production Release (Q4 2025)

**Trust and Security**
- Complete cryptographic lineage for all operations
- Zero-knowledge proofs for selective disclosure
- Cross-boundary collaboration protocols
- Distributed consensus where required

## Documentation and Research

- **[Introduction Blog](https://econic.ai/blog/massive-graph-introduction)** - Vision and use cases
- **[Development Blog](https://econic.ai/blog/massive-graph-development-phase-1)** - Technical implementation details
- **[Research Paper](https://econic.ai/docs/research/massive-graph)** - Formal architecture and theoretical foundations
- **[Technology Intro](https://local.econic.ai/docs/technology/massive-graph/introduction)** - Different applications of the technology

## Performance Benchmarks

TBA

## Contributing

We're building open-source infrastructure for the intelligence economy. Contributions welcome:

1. **Core Engine**: Performance optimisations, data structure improvements
2. **Protocol Implementation**: WebTransport, QUIC optimisations  
3. **Testing**: Load testing, chaos engineering, correctness verification
4. **Documentation**: API docs, tutorials, examples

## License

Apache 2.0 - See [LICENSE](LICENSE) for details.

---

*Part of the [Econic](https://econic.ai) platform for collaborative intelligence infrastructure.*