use ibc_test_framework::ibc::denom::derive_ibc_denom;
use ibc_test_framework::prelude::*;
use ibc_test_framework::util::random::random_u64_range;

#[test]
fn test_clear_packet() -> Result<(), Error> {
    run_binary_channel_test(&ClearPacketTest)
}

#[test]
fn test_clear_packet_recovery_dst() -> Result<(), Error> {
    run_binary_channel_test(&ClearPacketRecoveryDstTest)
}

#[test]
fn test_clear_packet_recovery_src() -> Result<(), Error> {
    run_binary_channel_test(&ClearPacketRecoverySrcTest)
}

pub struct ClearPacketTest;
pub struct ClearPacketRecoveryDstTest;
pub struct ClearPacketRecoverySrcTest;

impl TestOverrides for ClearPacketTest {
    fn modify_relayer_config(&self, config: &mut Config) {
        // Disabling clear_on_start should make the relayer not
        // relay any packet it missed before starting.
        config.mode.packets.clear_on_start = false;
        config.mode.packets.clear_interval = 0;
    }

    fn should_spawn_supervisor(&self) -> bool {
        false
    }

    // Unordered channel: will permit gaps in the sequence of relayed packets
    fn channel_order(&self) -> Order {
        Order::Unordered
    }
}

impl TestOverrides for ClearPacketRecoveryDstTest {
    fn modify_relayer_config(&self, config: &mut Config) {
        config.mode.packets.enabled = true;
        config.mode.packets.clear_on_start = true;
    }

    fn should_spawn_supervisor(&self) -> bool {
        false
    }
}

impl TestOverrides for ClearPacketRecoverySrcTest {
    fn modify_relayer_config(&self, config: &mut Config) {
        config.mode.packets.enabled = true;
        config.mode.packets.clear_on_start = true;
    }

    fn should_spawn_supervisor(&self) -> bool {
        false
    }
}

impl BinaryChannelTest for ClearPacketTest {
    fn run<ChainA: ChainHandle, ChainB: ChainHandle>(
        &self,
        _config: &TestConfig,
        relayer: RelayerDriver,
        chains: ConnectedChains<ChainA, ChainB>,
        channel: ConnectedChannel<ChainA, ChainB>,
    ) -> Result<(), Error> {
        let denom_a = chains.node_a.denom();

        let wallet_a = chains.node_a.wallets().user1().cloned();
        let wallet_b = chains.node_b.wallets().user1().cloned();

        let balance_a = chains
            .node_a
            .chain_driver()
            .query_balance(&wallet_a.address(), &denom_a)?;

        let amount1 = random_u64_range(1000, 5000);

        info!(
            "Performing IBC transfer with amount {}, which should *not* be relayed",
            amount1
        );

        chains.node_a.chain_driver().ibc_transfer_token(
            &channel.port_a.as_ref(),
            &channel.channel_id_a.as_ref(),
            &wallet_a.as_ref(),
            &wallet_b.address(),
            &denom_a,
            amount1,
        )?;

        sleep(Duration::from_secs(1));

        // Spawn the supervisor only after the first IBC trasnfer
        relayer.with_supervisor(|| {
            sleep(Duration::from_secs(1));

            let amount2 = random_u64_range(1000, 5000);

            info!(
                "Performing IBC transfer with amount {}, which should be relayed",
                amount2
            );

            chains.node_a.chain_driver().ibc_transfer_token(
                &channel.port_a.as_ref(),
                &channel.channel_id_a.as_ref(),
                &wallet_a.as_ref(),
                &wallet_b.address(),
                &denom_a,
                amount2,
            )?;

            sleep(Duration::from_secs(1));

            let denom_b = derive_ibc_denom(
                &channel.port_b.as_ref(),
                &channel.channel_id_b.as_ref(),
                &denom_a,
            )?;

            // Wallet on chain A should have both amount deducted.
            chains.node_a.chain_driver().assert_eventual_wallet_amount(
                &wallet_a.address(),
                balance_a - amount1 - amount2,
                &denom_a,
            )?;

            // Wallet on chain B should only receive the second IBC transfer
            chains.node_b.chain_driver().assert_eventual_wallet_amount(
                &wallet_b.address(),
                amount2,
                &denom_b.as_ref(),
            )?;

            Ok(())
        })
    }
}

impl BinaryChannelTest for ClearPacketRecoveryDstTest {
    fn run<ChainA: ChainHandle, ChainB: ChainHandle>(
        &self,
        _config: &TestConfig,
        relayer: RelayerDriver,
        chains: ConnectedChains<ChainA, ChainB>,
        channel: ConnectedChannel<ChainA, ChainB>,
    ) -> Result<(), Error> {
        let denom_a = chains.node_a.denom();
        let denom_b1 = chains.node_b.denom();

        let wallet_a = chains.node_a.wallets().user1().cloned();
        let wallet_b = chains.node_b.wallets().user1().cloned();

        let relayer_wallet_b = chains.node_b.wallets().relayer().cloned();

        // mess up the cached account sequence in ChainHandle of chain B
        // commenting out this local transfer would make the test pass
        chains.node_b.chain_driver().local_transfer_token(
            &relayer_wallet_b.as_ref(),
            &wallet_b.address(),
            100,
            &denom_b1,
        )?;

        let amount1 = random_u64_range(1000, 5000);

        chains.node_a.chain_driver().ibc_transfer_token(
            &channel.port_a.as_ref(),
            &channel.channel_id_a.as_ref(),
            &wallet_a.as_ref(),
            &wallet_b.address(),
            &denom_a,
            amount1,
        )?;

        let denom_b2 = derive_ibc_denom(
            &channel.port_b.as_ref(),
            &channel.channel_id_b.as_ref(),
            &denom_a,
        )?;

        relayer.with_supervisor(|| {
            chains.node_b.chain_driver().assert_eventual_wallet_amount(
                &wallet_b.address(),
                amount1,
                &denom_b2.as_ref(),
            )?;

            Ok(())
        })
    }
}

impl BinaryChannelTest for ClearPacketRecoverySrcTest {
    fn run<ChainA: ChainHandle, ChainB: ChainHandle>(
        &self,
        _config: &TestConfig,
        relayer: RelayerDriver,
        chains: ConnectedChains<ChainA, ChainB>,
        channel: ConnectedChannel<ChainA, ChainB>,
    ) -> Result<(), Error> {
        let denom_a = chains.node_a.denom();

        let wallet_a = chains.node_a.wallets().user1().cloned();
        let wallet_b = chains.node_b.wallets().user1().cloned();

        let relayer_wallet_a = chains.node_a.wallets().relayer().cloned();

        // mess up the cached account sequence in ChainHandle of chain A
        // commenting out this local transfer would make the test pass
        chains.node_a.chain_driver().local_transfer_token(
            &relayer_wallet_a.as_ref(),
            &wallet_a.address(),
            100,
            &denom_a,
        )?;

        let amount1 = random_u64_range(1000, 5000);

        chains.node_a.chain_driver().ibc_transfer_token(
            &channel.port_a.as_ref(),
            &channel.channel_id_a.as_ref(),
            &wallet_a.as_ref(),
            &wallet_b.address(),
            &denom_a,
            amount1,
        )?;

        let denom_b2 = derive_ibc_denom(
            &channel.port_b.as_ref(),
            &channel.channel_id_b.as_ref(),
            &denom_a,
        )?;

        relayer.with_supervisor(|| {
            chains.node_b.chain_driver().assert_eventual_wallet_amount(
                &wallet_b.address(),
                amount1,
                &denom_b2.as_ref(),
            )?;

            // The test would pass, but the ack probably fails.
            // Sleep to observe the failure in ack relaying.
            std::thread::sleep(Duration::from_secs(10));

            Ok(())
        })
    }
}
