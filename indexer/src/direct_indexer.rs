use std::{env, pin::Pin};

use environment::{Environment, EnvironmentBuilder, LoggerConfig};
use eth2_network_config::{Eth2NetworkConfig, DEFAULT_HARDCODED_NETWORK};
use futures::{Future, StreamExt};
use libp2p::{
    Multiaddr,
};
use lighthouse_network::{
    rpc::{BlocksByRootRequest},
    Request,
};
use store::{Hash256, MainnetEthSpec};

use crate::{beacon_node_client::BeaconNodeClient, network::network_service::NetworkService};


// use the executor for libp2p
struct Executor(task_executor::TaskExecutor);

impl libp2p::core::Executor for Executor {
    fn exec(&self, f: Pin<Box<dyn Future<Output = ()> + Send>>) {
        self.0.spawn(f, "libp2p");
    }
}

pub struct Indexer;

impl Indexer {
    pub fn start() -> Result<(), String> {
        let endpoint = env::var("ENDPOINT_URL").unwrap();
        let client = BeaconNodeClient::new(endpoint);
        let (mut environment, network_config) = Self::build_environment().unwrap();
        let context = environment.core_context();
        let executor = context.executor.clone();

        executor.spawn(
            async move {
                let peers = client
                    .get_connected_peers::<MainnetEthSpec>()
                    .await
                    .unwrap();

                //println!("Peers: {:?}", peers);

                let peer_id = "16Uiu2HAkwgkdraX5wvaCkuRi1YdU5VUvpdQH42Un2DXyADYXAD8Q".parse().unwrap();

                let remote: Multiaddr =
                    "/ip4/192.168.1.12/tcp/9000/p2p/16Uiu2HAkwgkdraX5wvaCkuRi1YdU5VUvpdQH42Un2DXyADYXAD8Q"
                        .parse()
                        .unwrap();

                let root: Hash256 =
                    "0x74442257e97a932c7b7427e4b0fa35b70e2e650fe6a4841ec22f706296797e70"
                        .parse()
                        .unwrap();

                let mut service = NetworkService::new(context.clone(), network_config).await.unwrap();

                service.connect(peer_id, &remote).await;

                /*
                service
                    .send_request(
                        Request::BlocksByRoot(BlocksByRootRequest {
                            block_roots: vec![root].into(),
                        }),
                        peer_id,
                    )
                    .await.unwrap();
                */
                loop {
                    service.next().await;
                }
                
            },
            "network",
        );

        environment.block_until_shutdown_requested().unwrap();

        Ok(())
    }

    fn build_environment() -> Result<(Environment<MainnetEthSpec>, Eth2NetworkConfig), String> {
        let environment_builder = EnvironmentBuilder::mainnet();
        let logger_config = LoggerConfig {
            path: None,
            debug_level: "info",
            logfile_debug_level: "info",
            log_format: None,
            max_log_size: 0,
            max_log_number: 0,
            compression: false,
        };
        let eth2_network_config = Eth2NetworkConfig::constant(DEFAULT_HARDCODED_NETWORK)?
            .ok_or("Failed to build Eth2 network config")?;
        let environment = environment_builder
            .eth2_network_config(eth2_network_config.clone())?
            .initialize_logger(logger_config)?
            .multi_threaded_tokio_runtime()?
            .build()?;

        Ok((environment, eth2_network_config))
    }
}
