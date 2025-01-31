use abscissa_core::clap::Parser;
use abscissa_core::{Command, Runnable};

use ibc::core::ics24_host::identifier::ChainId;
use ibc_relayer::chain::handle::ChainHandle;

use crate::application::app_config;
use crate::cli_utils::spawn_chain_runtime;
use crate::conclude::{exit_with_unrecoverable_error, json, Output};

/// The data structure that represents the arguments when invoking the `keys balance` CLI command.
///
/// The command has one argument and one optional flag:
///
/// `keys balance <chain_id> --key-name <KEY_NAME>`
///
/// If no key name is given, it will be taken from the configuration file.
/// If successful the balance and denominator of the account, associated with the key name
/// on the given chain, will be displayed.
#[derive(Clone, Command, Debug, Parser)]
pub struct KeyBalanceCmd {
    #[clap(required = true, help = "identifier of the chain")]
    chain_id: ChainId,

    #[clap(
        long,
        short,
        help = "(optional) name of the key (defaults to the `key_name` defined in the config)"
    )]
    key_name: Option<String>,
}

impl Runnable for KeyBalanceCmd {
    fn run(&self) {
        let config = app_config();

        let chain = spawn_chain_runtime(&config, &self.chain_id)
            .unwrap_or_else(exit_with_unrecoverable_error);
        let key_name = self.key_name.clone();

        match chain.query_balance(key_name.clone()) {
            Ok(balance) if json() => Output::success(balance).exit(),
            Ok(balance) => {
                // Retrieve the key name string to output.
                let key_name_str = match key_name {
                    Some(name) => name,
                    None => {
                        let chain_config =
                            chain.config().unwrap_or_else(exit_with_unrecoverable_error);
                        chain_config.key_name
                    }
                };
                Output::success_msg(format!(
                    "balance for key `{}`: {} {}",
                    key_name_str, balance.amount, balance.denom
                ))
                .exit()
            }
            Err(e) => Output::error(format!(
                "there was a problem querying the chain balance: {}",
                e
            ))
            .exit(),
        }
    }
}
