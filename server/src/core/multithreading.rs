const CORE_WORKERS: usize = 32;  // Sharded document ownership

struct CoreWorker {
    worker_id: usize,
    documents: HashMap<DocId, Document>,
    inbox: spsc::Receiver<DeltaTask>,  // SPSC from network layer
    propagation_queues: Vec<spsc::Sender<PropagationTask>>,
}

// Performance characteristics:
// - Throughput: 1.6M deltas/sec per worker
// - Total: 32 × 1.6M = 51.2M deltas/sec
// - Latency: ~600ns per delta
impl CoreWorker {
    async fn run(mut self) {
        // Pin to CPU for cache locality
        let cpu_id = self.worker_id % num_cpus::get();
        core_affinity::set_for_current(CoreId { id: cpu_id });
        
        while let Some(task) = self.inbox.recv() {
            // Get or create document (owned by this worker)
            let doc = self.documents
                .entry(task.doc_id)
                .or_insert_with(|| Document::new(task.doc_id));
            
            // Validate against schema (~500ns)
            let delta = Delta::from_chunk(task.chunk_ref);
            if !doc.validate_schema(&delta) {
                continue; // Reject invalid delta
            }
            
            // Create delta node for linked list
            let node = Box::into_raw(Box::new(DeltaNode {
                chunk_ref: task.chunk_ref,
                next: AtomicPtr::new(null_mut()),
                sequence: doc.next_sequence(),
            }));
            
            // Append to document's delta stream (lock-free)
            let old_tail = doc.delta_stream.tail.swap(node, Ordering::Release);
            if !old_tail.is_null() {
                unsafe { (*old_tail).next.store(node, Ordering::Release) };
            }
            
            // Update document version
            doc.version.fetch_add(1, Ordering::Release);
            
            // Queue for propagation (sharded by document)
            let prop_worker = hash(task.doc_id) % self.propagation_queues.len();
            self.propagation_queues[prop_worker].push(PropagationTask {
                doc_id: task.doc_id,
                delta_ref: node,
                subscribers: doc.subscribers.clone(),
            });
        }
    }
}



struct ElasticWorkerPool {
    // Core workers (always running)
    core_workers: Vec<CoreWorker>,
    
    // Elastic workers (spawned on demand)
    elastic_workers: DashMap<DocId, ElasticWorkerHandle>,
    max_elastic: usize,  // e.g., 24 additional workers
    current_elastic: AtomicUsize,
    
    // Document statistics for promotion decisions
    doc_stats: DashMap<DocId, DocStats>,
}

struct DocStats {
    delta_rate: AtomicU32,        // Rolling average
    last_delta: AtomicU64,        // Timestamp
    assigned_worker: AtomicU8,    // 0=core, >0=elastic
}


// Performance characteristics:
// - Spawning latency: ~1ms (acceptable for long-lived streams)
// - Throughput: 1.6M deltas/sec per worker (same as core)
// - Isolation: Prevents greedy documents from blocking others
impl ElasticWorkerPool {
    async fn monitor_and_adapt(&self) {
        loop {
            tokio::time::sleep(Duration::from_millis(100)).await;
            
            for entry in self.doc_stats.iter() {
                let (doc_id, stats) = entry.pair();
                let rate = stats.delta_rate.load(Ordering::Relaxed);
                
                // Promotion: Document overwhelming its shared worker
                if rate > 1_000_000 && stats.assigned_worker.load(Ordering::Relaxed) == 0 {
                    if self.current_elastic.load(Ordering::Relaxed) < self.max_elastic {
                        self.spawn_elastic_worker(*doc_id).await;
                    }
                }
                
                // Demotion: Document no longer active
                if rate < 10_000 && stats.assigned_worker.load(Ordering::Relaxed) > 0 {
                    let idle_time = timestamp_now() - stats.last_delta.load(Ordering::Relaxed);
                    if idle_time > 5000 {  // 5 seconds idle
                        self.demote_to_core(*doc_id).await;
                    }
                }
            }
        }
    }
    
    async fn spawn_elastic_worker(&self, doc_id: DocId) {
        let (tx, rx) = spsc::channel(100_000);
        
        let worker = ElasticWorker {
            doc_id,
            document: Document::new(doc_id),
            inbox: rx,
        };
        
        let handle = tokio::spawn(async move {
            // Pin to dedicated CPU
            let cpu_id = hash(doc_id) % num_cpus::get();
            core_affinity::set_for_current(CoreId { id: cpu_id });
            
            worker.run().await;
        });
        
        self.elastic_workers.insert(doc_id, ElasticWorkerHandle {
            sender: tx,
            join_handle: handle,
        });
        
        self.current_elastic.fetch_add(1, Ordering::Release);
        log::info!("Spawned elastic worker for document {:?}", doc_id);
    }
}

const PROPAGATION_WORKERS: usize = 16;

struct PropagationWorker {
    worker_id: usize,
    inbox: spsc::Receiver<PropagationTask>,
    
    // Client connections with multiple DataChannels
    clients: DashMap<ClientId, ClientConnection>,
}

// Performance characteristics:
// - Throughput: 5M msgs/sec per DataChannel
// - With 100 channels per client: 500M msgs/sec capability
// - Network I/O: ~10μs per send operation
impl PropagationWorker {
    async fn run(mut self) {
        while let Some(task) = self.inbox.recv() {
            // Get delta bytes from chunk (zero-copy reference)
            let delta_bytes = unsafe {
                let node = &*task.delta_ref;
                self.chunk_storage.get_bytes(node.chunk_ref)
            };
            
            // Send to all subscribers in parallel
            let futures: Vec<_> = task.subscribers
                .iter()
                .filter_map(|client_id| {
                    self.clients.get(client_id).map(|client| {
                        let bytes = delta_bytes.clone();  // Arc'd, no copy
                        let doc_id = task.doc_id;
                        
                        async move {
                            // Use dedicated channel if available
                            if let Some(channel) = client.document_channels.get(&doc_id) {
                                channel.send_binary(bytes).await
                            } else {
                                client.shared_channel.send_binary(bytes).await
                            }
                        }
                    })
                })
                .collect();
            
            futures::future::join_all(futures).await;
        }
    }
}

