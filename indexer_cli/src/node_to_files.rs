use std::fs;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use indexer::node_to_files_indexer::Indexer;
use indexer::retriever::Retriever;

use crate::cli::Cli;

pub async fn process(cli: Cli) {
    if cli.reset {
        fs::remove_dir_all("../web/public/data").unwrap();
    }

    fs::create_dir_all("../web/public/data/epochs/e/").unwrap();
    fs::create_dir_all("../web/public/data/epochs/s/attestations_count/").unwrap();
    fs::create_dir_all("../web/public/data/epochs/s/deposits_count/").unwrap();
    fs::create_dir_all("../web/public/data/epochs/s/attester_slashings_count/").unwrap();
    fs::create_dir_all("../web/public/data/epochs/s/proposer_slashings_count/").unwrap();
    fs::create_dir_all("../web/public/data/epochs/s/eligible_ether/").unwrap();
    fs::create_dir_all("../web/public/data/epochs/s/voted_ether/").unwrap();
    fs::create_dir_all("../web/public/data/epochs/s/global_participation_rate/").unwrap();
    fs::create_dir_all("../web/public/data/blocks").unwrap();
    fs::create_dir_all("../web/public/data/blocks/e/").unwrap();
    fs::create_dir_all("../web/public/data/blocks/a/").unwrap();
    fs::create_dir_all("../web/public/data/blocks/c/").unwrap();
    fs::create_dir_all("../web/public/data/blocks/v/").unwrap();
    fs::create_dir_all("../web/public/data/validators").unwrap();

    let running = Arc::new(AtomicBool::new(true));

    let (sender, receiver) = oneshot::channel::<()>();
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    tokio::spawn(async move {
        let retriever = retrieve(running);
        let indexer = Indexer::from(retriever);
    
        indexer.index("../web/public/data").unwrap();

        sender.send(()).unwrap();
    });

    receiver.await.unwrap();

}


async fn retrieve(running: Arc<AtomicBool>) -> Retriever {
    let mut retriever = Retriever::new(cli.endpoint_url);
    let mut n = 0;

    while running.load(Ordering::SeqCst) {
        match retriever.retrieve_epoch(n).await {
            Ok(_) => {
                n += 1;
            }
            Err(err) => {
                running.store(false, Ordering::SeqCst);
                log::error!("Error while retrieving epoch {}: {:?}", n, err);
            }
        }
    }

    match retriever.retrieve_validators().await {
        Ok(_) => (),
        Err(err) => {
            log::error!("Error while retrieving validators: {:?}", err);
        }
    }

    retriever
}