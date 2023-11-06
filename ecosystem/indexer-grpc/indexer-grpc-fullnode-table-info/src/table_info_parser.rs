// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::convert::{QUERY_RETRIES, QUERY_RETRY_DELAY_MS};
use anyhow::{bail, ensure, Result, format_err};
use aptos_config::config::RocksdbConfig;
use aptos_db_indexer::schema::{
    column_families, table_info::TableInfoSchema,
};
use aptos_logger::info;
use aptos_rocksdb_options::gen_rocksdb_options;
use aptos_schemadb::{SchemaBatch, DB};
use aptos_sdk::bcs;
use aptos_types::{
    access_path::Path,
    account_address::AccountAddress,
    state_store::{
        state_key::{StateKey, StateKeyInner},
        table::{TableHandle, TableInfo},
    },
    transaction::Version,
    write_set::{WriteOp, WriteSet},
};
use bytes::Bytes;
use move_core_types::{
    ident_str,
    language_storage::{StructTag, TypeTag},
    resolver::MoveResolver,
};
use move_resource_viewer::{AnnotatedMoveValue, MoveValueAnnotator};
use std::{
    collections::{BTreeMap, HashMap},
    convert::TryInto,
};
use aptos_storage_interface::DbReader;

pub const INDEXER_LOOKUP_DB_NAME: &str = "indexer_lookup_db";

#[derive(Debug)]
pub struct IndexerLookupDB {
    db: DB,
}

impl IndexerLookupDB {
    pub fn open(
        db_root_path: impl AsRef<std::path::Path>,
        rocksdb_config: RocksdbConfig,
    ) -> Result<Self> {
        let db_path = db_root_path.as_ref().join(INDEXER_LOOKUP_DB_NAME);

        let db = DB::open(
            db_path,
            INDEXER_LOOKUP_DB_NAME,
            column_families(),
            &gen_rocksdb_options(&rocksdb_config, false),
        )?;

        Ok(Self { db })
    }

    pub fn index_with_annotator<R: MoveResolver>(
        &self,
        annotator: &MoveValueAnnotator<'_, R>,
        first_version: Version,
        write_sets: &[&WriteSet],
    ) -> Result<()> {
        let end_version = first_version + write_sets.len() as Version;
        let mut table_info_parser = TableInfoParser::new(self, annotator);
        for write_set in write_sets {
            for (state_key, write_op) in write_set.iter() {
                table_info_parser.parse_write_op(state_key, write_op)?;
            }
        }

        let mut batch = SchemaBatch::new();

        match table_info_parser.finish(&mut batch) {
            Ok(_) => {
                info!("table info batch finished processing");
            },
            Err(err) => {
                aptos_logger::error!(first_version = first_version, end_version = end_version, error = ?&err);
                write_sets
                .iter()
                .enumerate()
                .for_each(|(i, write_set)| {
                    aptos_logger::error!(version = first_version as usize + i, write_set = ?write_set);
                });
                bail!(err);
            },
        };
        self.db.write_schemas(batch)?;
        Ok(())
    }

    pub fn get_table_info(&self, handle: TableHandle) -> Result<Option<TableInfo>> {
        let mut retried = 0;
        while retried < QUERY_RETRIES {
            retried += 1;
            if let Ok(result) = self.db.get::<TableInfoSchema>(&handle) {
                if let Some(table_info) = result {
                    return Ok(Some(table_info));
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(QUERY_RETRY_DELAY_MS));
        }
        Ok(None)
    }    
}

struct TableInfoParser<'a, R> {
    indexer: &'a IndexerLookupDB,
    annotator: &'a MoveValueAnnotator<'a, R>,
    result: HashMap<TableHandle, TableInfo>,
    pending_on: HashMap<TableHandle, Vec<Bytes>>,
}

impl<'a, R: MoveResolver> TableInfoParser<'a, R> {
    pub fn new(indexer: &'a IndexerLookupDB, annotator: &'a MoveValueAnnotator<R>) -> Self {
        Self {
            indexer,
            annotator,
            result: HashMap::new(),
            pending_on: HashMap::new(),
        }
    }

    pub fn parse_write_op(&mut self, state_key: &'a StateKey, write_op: &'a WriteOp) -> Result<()> {
        if let Some(bytes) = write_op.bytes() {
            match state_key.inner() {
                StateKeyInner::AccessPath(access_path) => {
                    let path: Path = (&access_path.path).try_into()?;
                    match path {
                        Path::Code(_) => (),
                        Path::Resource(struct_tag) => self.parse_struct(struct_tag, bytes)?,
                        Path::ResourceGroup(_struct_tag) => self.parse_resource_group(bytes)?,
                    }
                },
                StateKeyInner::TableItem { handle, .. } => self.parse_table_item(*handle, bytes)?,
                StateKeyInner::Raw(_) => (),
            }
        }
        Ok(())
    }

    fn parse_struct(&mut self, struct_tag: StructTag, bytes: &Bytes) -> Result<()> {
        self.parse_move_value(
            &self
                .annotator
                .view_value(&TypeTag::Struct(Box::new(struct_tag)), bytes)?,
        )
    }

    fn parse_resource_group(&mut self, bytes: &Bytes) -> Result<()> {
        type ResourceGroup = BTreeMap<StructTag, Bytes>;

        for (struct_tag, bytes) in bcs::from_bytes::<ResourceGroup>(bytes)? {
            self.parse_struct(struct_tag, &bytes)?;
        }
        Ok(())
    }

    fn parse_table_item(&mut self, handle: TableHandle, bytes: &Bytes) -> Result<()> {
        match self.get_table_info(handle)? {
            Some(table_info) => {
                self.parse_move_value(&self.annotator.view_value(&table_info.value_type, bytes)?)?;
            },
            None => {
                self.pending_on
                    .entry(handle)
                    .or_insert_with(Vec::new)
                    .push(bytes.clone());
            },
        }
        Ok(())
    }

    fn parse_move_value(&mut self, move_value: &AnnotatedMoveValue) -> Result<()> {
        match move_value {
            AnnotatedMoveValue::Vector(_type_tag, items) => {
                for item in items {
                    self.parse_move_value(item)?;
                }
            },
            AnnotatedMoveValue::Struct(struct_value) => {
                let struct_tag = &struct_value.type_;
                if Self::is_table(struct_tag) {
                    assert_eq!(struct_tag.type_params.len(), 2);
                    let table_info = TableInfo {
                        key_type: struct_tag.type_params[0].clone(),
                        value_type: struct_tag.type_params[1].clone(),
                    };
                    let table_handle = match &struct_value.value[0] {
                        (name, AnnotatedMoveValue::Address(handle)) => {
                            assert_eq!(name.as_ref(), ident_str!("handle"));
                            TableHandle(*handle)
                        },
                        _ => bail!("Table struct malformed. {:?}", struct_value),
                    };
                    self.save_table_info(table_handle, table_info)?;
                } else {
                    for (_identifier, field) in &struct_value.value {
                        self.parse_move_value(field)?;
                    }
                }
            },

            // there won't be tables in primitives
            AnnotatedMoveValue::U8(_) => {},
            AnnotatedMoveValue::U16(_) => {},
            AnnotatedMoveValue::U32(_) => {},
            AnnotatedMoveValue::U64(_) => {},
            AnnotatedMoveValue::U128(_) => {},
            AnnotatedMoveValue::U256(_) => {},
            AnnotatedMoveValue::Bool(_) => {},
            AnnotatedMoveValue::Address(_) => {},
            AnnotatedMoveValue::Bytes(_) => {},
        }
        Ok(())
    }

    fn save_table_info(&mut self, handle: TableHandle, info: TableInfo) -> Result<()> {
        if self.get_table_info(handle)?.is_none() {
            self.result.insert(handle, info);
            if let Some(pending_items) = self.pending_on.remove(&handle) {
                for bytes in pending_items {
                    self.parse_table_item(handle, &bytes)?;
                }
            }
        }
        Ok(())
    }

    fn is_table(struct_tag: &StructTag) -> bool {
        struct_tag.address == AccountAddress::ONE
            && struct_tag.module.as_ident_str() == ident_str!("table")
            && struct_tag.name.as_ident_str() == ident_str!("Table")
    }

    fn get_table_info(&self, handle: TableHandle) -> Result<Option<TableInfo>> {
        match self.result.get(&handle) {
            Some(table_info) => Ok(Some(table_info.clone())),
            None => self.indexer.get_table_info(handle),
        }
    }

    fn finish(self, batch: &mut SchemaBatch) -> Result<bool> {
        ensure!(
            self.pending_on.is_empty(),
            "There is still pending table items to parse due to unknown table info for table handles: {:?}",
            self.pending_on.keys(),
        );
        if self.result.is_empty() {
            Ok(false)
        } else {
            self.result
                .into_iter()
                .try_for_each(|(table_handle, table_info)| {
                    batch.put::<TableInfoSchema>(&table_handle, &table_info)
                })?;
            Ok(true)
        }
    }
}

impl DbReader for IndexerLookupDB {
    fn get_table_info(&self, handle: TableHandle) -> Result<TableInfo> {
        Self::get_table_info(self, handle)?.ok_or_else(|| format_err!("TableInfo for {:?}", handle))
    }
}
