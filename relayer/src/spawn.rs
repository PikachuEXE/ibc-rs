use alloc::sync::Arc;

use flex_error::define_error;
use tokio::runtime::Runtime as TokioRuntime;

use ibc::core::ics24_host::identifier::ChainId;

use crate::{
    chain::{cosmos::CosmosSdkChain, handle::ChainHandle, runtime::ChainRuntime, ChainType},
    config::Config,
    error::Error as RelayerError,
};

#[cfg(test)]
use crate::chain::mock::MockChain;

define_error! {
    SpawnError {
        Relayer
            [ RelayerError ]
            | _ | { "relayer error" },

        RuntimeNotFound
            | _ | { "expected runtime to be found in registry" },

        MissingChainConfig
            { chain_id: ChainId }
            | e | {
                format_args!("missing chain config for '{}' in configuration file", e.chain_id)
            }
    }
}

impl SpawnError {
    pub fn log_as_debug(&self) -> bool {
        self.detail().log_as_debug()
    }
}

impl SpawnErrorDetail {
    pub fn log_as_debug(&self) -> bool {
        matches!(self, SpawnErrorDetail::MissingChainConfig(_))
    }
}

/// Spawns a chain runtime from the configuration and given a chain identifier.
/// Returns the corresponding handle if successful.
pub fn spawn_chain_runtime<Handle: ChainHandle>(
    config: &Config,
    chain_id: &ChainId,
    rt: Arc<TokioRuntime>,
) -> Result<Handle, SpawnError> {
    let chain_config = config
        .find_chain(chain_id)
        .cloned()
        .ok_or_else(|| SpawnError::missing_chain_config(chain_id.clone()))?;

    dbg!(chain_config.r#type);

    let handle = match chain_config.r#type {
        ChainType::CosmosSdk => ChainRuntime::<CosmosSdkChain>::spawn::<Handle>(chain_config, rt),

        #[cfg(test)]
        ChainType::Mock => ChainRuntime::<MockChain>::spawn::<Handle>(chain_config, rt),
    }
    .map_err(SpawnError::relayer)?;

    Ok(handle)
}
