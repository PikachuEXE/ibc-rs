use core::str::FromStr;

use abscissa_core::clap::Parser;
use abscissa_core::{Command, Runnable};
use tracing::debug;

use tendermint::abci::transaction::Hash;

use ibc::core::ics24_host::identifier::ChainId;
use ibc::query::{QueryTxHash, QueryTxRequest};

use ibc_relayer::chain::handle::ChainHandle;

use crate::cli_utils::spawn_chain_runtime;
use crate::conclude::{exit_with_unrecoverable_error, Output};
use crate::error::Error;
use crate::prelude::app_config;

/// Query the events emitted by transaction
#[derive(Clone, Command, Debug, Parser)]
pub struct QueryTxEventsCmd {
    #[clap(required = true, help = "identifier of the chain to query")]
    chain_id: ChainId,

    #[clap(required = true, help = "transaction hash to query")]
    hash: String,
}

// cargo run --bin hermes -- query tx events ibc-0 B8E78AD83810239E21863AC7B5FC4F99396ABB39EB534F721EEF43A4979C2821
impl Runnable for QueryTxEventsCmd {
    fn run(&self) {
        let config = app_config();

        debug!("Options: {:?}", self);

        let chain = spawn_chain_runtime(&config, &self.chain_id)
            .unwrap_or_else(exit_with_unrecoverable_error);

        let res = Hash::from_str(self.hash.as_str())
            .map_err(|e| Error::invalid_hash(self.hash.clone(), e))
            .and_then(|h| {
                chain
                    .query_txs(QueryTxRequest::Transaction(QueryTxHash(h)))
                    .map_err(Error::relayer)
            });

        match res {
            Ok(res) => Output::success(res).exit(),
            Err(e) => Output::error(format!("{}", e)).exit(),
        }
    }
}
