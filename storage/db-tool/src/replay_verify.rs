// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_backup_cli::{
    coordinators::replay_verify::{ReplayError, ReplayVerifyCoordinator},
    metadata::cache::MetadataCacheOpt,
    storage::DBToolStorageOpt,
    utils::{ConcurrentDownloadsOpt, ReplayConcurrencyLevelOpt, RocksdbOpt, TrustedWaypointOpt},
};
use aptos_config::config::{
    StorageDirPaths, BUFFERED_STATE_TARGET_ITEMS, DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
    NO_OP_STORAGE_PRUNER_CONFIG,
};
use aptos_db::{AptosDB, GetRestoreHandler};
use aptos_executor_types::VerifyExecutionMode;
use aptos_logger::info;
use aptos_types::transaction::Version;
use clap::Parser;
use std::{path::PathBuf, process, sync::Arc};

/// Read the backup files, replay them and verify the modules
#[derive(Parser)]
pub struct Opt {
    #[clap(flatten)]
    metadata_cache_opt: MetadataCacheOpt,
    #[clap(flatten)]
    trusted_waypoints_opt: TrustedWaypointOpt,
    #[clap(flatten)]
    storage: DBToolStorageOpt,
    #[clap(flatten)]
    concurrent_downloads: ConcurrentDownloadsOpt,
    #[clap(flatten)]
    replay_concurrency_level: ReplayConcurrencyLevelOpt,
    #[clap(long = "target-db-dir", value_parser)]
    pub db_dir: PathBuf,
    #[clap(flatten)]
    pub rocksdb_opt: RocksdbOpt,
    #[clap(
        long,
        help = "The first transaction version required to be replayed and verified. [Defaults to 0]"
    )]
    start_version: Option<Version>,
    #[clap(
        long,
        help = "The last transaction version required to be replayed and verified (if present \
        in the backup). [Defaults to the latest version available] "
    )]
    end_version: Option<Version>,
    #[clap(long)]
    validate_modules: bool,
    #[clap(
        long,
        num_args = 1..,
        help = "Skip the execution for txns that are known to break compatibility."
    )]
    txns_to_skip: Vec<Version>,
    #[clap(long, help = "Do not quit right away when a replay issue is detected.")]
    lazy_quit: bool,
}

impl Opt {
    pub async fn run(self) -> Result<()> {
        let restore_handler = Arc::new(AptosDB::open_kv_only(
            StorageDirPaths::from_path(self.db_dir.clone()),
            false,                       /* read_only */
            NO_OP_STORAGE_PRUNER_CONFIG, /* pruner config */
            self.rocksdb_opt.clone().into(),
            false,
            BUFFERED_STATE_TARGET_ITEMS,
            DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
        )?)
        .get_restore_handler();
        let ret = ReplayVerifyCoordinator::new(
            self.storage.clone().init_storage().await?,
            self.metadata_cache_opt.clone(),
            self.trusted_waypoints_opt.clone(),
            self.concurrent_downloads.get(),
            self.replay_concurrency_level.get(),
            restore_handler.clone(),
            self.start_version.unwrap_or(0),
            self.end_version.unwrap_or(Version::MAX),
            self.validate_modules,
            VerifyExecutionMode::verify_except(self.txns_to_skip.clone())
                .set_lazy_quit(self.lazy_quit),
        )?
        .run()
        .await;
        match ret {
            Err(e) => match e {
                ReplayError::TxnMismatch => {
                    info!("ReplayVerify coordinator exiting with Txn output mismatch error.");
                    process::exit(2);
                },
                _ => {
                    process::exit(1);
                },
            },
            _ => {
                info!("ReplayVerify coordinator succeeded");
            },
        };
        Ok(())
    }
}
