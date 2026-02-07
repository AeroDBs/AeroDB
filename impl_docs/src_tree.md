.
├── api
│   ├── errors.rs
│   ├── handler.rs
│   ├── mod.rs
│   ├── request.rs
│   └── response.rs
├── backup
│   ├── archive.rs
│   ├── errors.rs
│   ├── manifest.rs
│   ├── mod.rs
│   └── packer.rs
├── checkpoint
│   ├── coordinator.rs
│   ├── errors.rs
│   ├── marker.rs
│   ├── mod.rs
│   └── pipeline.rs
├── cli
│   ├── args.rs
│   ├── commands.rs
│   ├── errors.rs
│   ├── io.rs
│   └── mod.rs
├── crash_point.rs
├── dx
│   ├── api
│   │   ├── handlers.rs
│   │   ├── mod.rs
│   │   ├── response.rs
│   │   └── server.rs
│   ├── config.rs
│   ├── explain
│   │   ├── checkpoint.rs
│   │   ├── model.rs
│   │   ├── mod.rs
│   │   ├── query.rs
│   │   ├── recovery.rs
│   │   ├── replication.rs
│   │   ├── rules.rs
│   │   └── visibility.rs
│   └── mod.rs
├── executor
│   ├── errors.rs
│   ├── executor.rs
│   ├── filters.rs
│   ├── mod.rs
│   ├── result.rs
│   └── sorter.rs
├── index
│   ├── acceleration.rs
│   ├── btree.rs
│   ├── errors.rs
│   ├── manager.rs
│   └── mod.rs
├── lib.rs
├── main.rs
├── mvcc
│   ├── commit_authority.rs
│   ├── commit_id.rs
│   ├── gc.rs
│   ├── mod.rs
│   ├── read_cache.rs
│   ├── read_view.rs
│   ├── version_chain.rs
│   ├── version.rs
│   ├── version_storage.rs
│   └── visibility.rs
├── observability
│   ├── events.rs
│   ├── logger.rs
│   ├── metrics.rs
│   ├── mod.rs
│   └── scope.rs
├── performance
│   ├── memory_layout.rs
│   └── mod.rs
├── planner
│   ├── ast.rs
│   ├── bounds.rs
│   ├── errors.rs
│   ├── explain.rs
│   ├── mod.rs
│   └── planner.rs
├── promotion
│   ├── controller.rs
│   ├── crash_tests.rs
│   ├── errors.rs
│   ├── integration.rs
│   ├── mod.rs
│   ├── observability.rs
│   ├── request.rs
│   ├── state.rs
│   ├── transition.rs
│   └── validator.rs
├── recovery
│   ├── adapters.rs
│   ├── errors.rs
│   ├── mod.rs
│   ├── replay.rs
│   ├── startup.rs
│   └── verifier.rs
├── replication
│   ├── authority.rs
│   ├── compatibility.rs
│   ├── config.rs
│   ├── errors.rs
│   ├── failure_matrix.rs
│   ├── fast_read.rs
│   ├── mod.rs
│   ├── recovery.rs
│   ├── replica_reads.rs
│   ├── role.rs
│   ├── snapshot_transfer.rs
│   ├── wal_receiver.rs
│   └── wal_sender.rs
├── restore
│   ├── errors.rs
│   ├── extractor.rs
│   ├── mod.rs
│   ├── restorer.rs
│   └── validator.rs
├── schema
│   ├── errors.rs
│   ├── loader.rs
│   ├── mod.rs
│   ├── types.rs
│   └── validator.rs
├── snapshot
│   ├── checksum.rs
│   ├── creator.rs
│   ├── errors.rs
│   ├── manifest.rs
│   └── mod.rs
├── src_tree.md
├── storage
│   ├── checksum.rs
│   ├── errors.rs
│   ├── mod.rs
│   ├── reader.rs
│   ├── record.rs
│   └── writer.rs
└── wal
    ├── batching.rs
    ├── checksum.rs
    ├── errors.rs
    ├── group_commit.rs
    ├── mod.rs
    ├── reader.rs
    ├── record.rs
    └── writer.rs

22 directories, 130 files
