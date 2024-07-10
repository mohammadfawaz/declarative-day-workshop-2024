use counter_app::App;
use essential_app_utils::{compile::compile_pint_project, local_server::setup_server};
use essential_types::contract::Contract;
use essential_types::{PredicateAddress, Word};

#[tokio::test]
async fn test_counter() {
    let (server_address, _server) = setup_server().await.unwrap();

    // Compile the Pint contract and return the contract object
    let counter_contract: Contract =
        compile_pint_project(concat!(env!("CARGO_MANIFEST_DIR"), "/../contract").into())
            .await
            .unwrap();

    // This is the address of the `Increment` predicate, which contans the address (hash) of the
    // predicate itself as well as the address (hash) of the counter contract.
    let increment_predicate_address = PredicateAddress {
        contract: essential_hash::contract_addr::from_contract(&counter_contract),
        predicate: essential_hash::content_addr(&counter_contract.predicates[0]),
    };

    // Set up wallet with name "alice"
    let mut wallet = essential_wallet::Wallet::temp().unwrap();
    wallet
        .new_key_pair("alice", essential_wallet::Scheme::Secp256k1)
        .unwrap();

    // Sign and deploy the counter contract at `server_address`
    essential_deploy_contract::sign_and_deploy(
        server_address.clone(),
        "alice",
        &mut wallet,
        counter_contract,
    )
    .await
    .unwrap();

    // This is a new instance of the counter app
    let app = App::new(server_address, increment_predicate_address).unwrap();

    // Ensure that the counter starts at 0
    assert_eq!(app.read_current_counter().await.unwrap(), 0);

    // Increment once
    app.increment().await.unwrap();
    check_new_counter_value(&app, 1).await;

    // Increment again
    app.increment().await.unwrap();
    check_new_counter_value(&app, 2).await;
}

/// Keep reading the current value of the counter until it changes.
async fn check_new_counter_value(app: &App, expected: Word) {
    loop {
        if app.read_current_counter().await.unwrap() == expected {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
