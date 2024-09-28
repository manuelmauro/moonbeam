// Copyright 2024 Moonbeam foundation
// This file is part of Moonbeam.

// Moonbeam is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Moonbeam is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Moonbeam.  If not, see <http://www.gnu.org/licenses/>.

use crate::lazy_loading;
use crate::lazy_loading::backend::RPC;
use cumulus_primitives_core::BlockT;
use sc_client_api::{Backend, BlockImportOperation, NewBlockState};
use sp_core::{twox_128, H256};
use sp_runtime::traits::{Header, One};
use sp_runtime::Saturating;
use sp_storage::{StateVersion, Storage, StorageKey};
use std::sync::Arc;

pub fn produce_genesis_block<TBl: BlockT + sp_runtime::DeserializeOwned>(
	backend: Arc<lazy_loading::backend::Backend<TBl>>,
) -> sp_blockchain::Result<()> {
	let mut op = backend.begin_operation()?;
	op.before_fork = true;

	let genesis_block_hash: TBl::Hash = backend
		.rpc_client
		.block_hash::<TBl>(Some(0))
		.unwrap()
		.expect("Not able to obtain genesis block hash");

	let genesis_block = backend
		.rpc_client
		.block::<TBl, _>(Some(genesis_block_hash))
		.unwrap()
		.unwrap()
		.block;

	let _ = op.set_block_data(
		genesis_block.header().clone(),
		Some(genesis_block.extrinsics().to_vec()),
		None,
		None,
		NewBlockState::Final,
	);

	backend.commit_operation(op)
}

pub fn produce_first_block<Block: BlockT + sp_runtime::DeserializeOwned>(
	backend: Arc<lazy_loading::backend::Backend<Block>>,
	state_overrides: Vec<(Vec<u8>, Vec<u8>)>,
) -> sp_blockchain::Result<()> {
	use sc_client_api::HeaderBackend;
	let mut op = backend.begin_operation()?;

	let state_root = op.reset_storage(
		Storage {
			top: state_overrides.into_iter().collect(),
			children_default: Default::default(),
		},
		StateVersion::V0,
	)?;

	let head_info = backend.blockchain.info();
	let next_block_number = head_info.finalized_number.saturating_add(One::one());

	let header: Block::Header = Block::Header::new(
		next_block_number,
		Default::default(),
		state_root,
		head_info.finalized_hash,
		Default::default(),
	);

	let _ = op.set_block_data(
		header.clone(),
		Some(Default::default()),
		None,
		None,
		NewBlockState::Final,
	);

	backend.commit_operation(op)
}

pub fn get_parachain_id(rpc_client: Arc<RPC>) -> Option<u32> {
	let key = [twox_128(b"ParachainInfo"), twox_128(b"ParachainId")].concat();
	let result = rpc_client.storage::<H256>(StorageKey(key), None);

	result
		.map(|o| {
			o.and_then(|data| {
				<u32 as parity_scale_codec::Decode>::decode(&mut data.0.as_slice()).ok()
			})
		})
		.ok()
		.flatten()
}