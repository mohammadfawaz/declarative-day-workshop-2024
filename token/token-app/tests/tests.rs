use essential_app_utils::{compile::compile_pint_project, local_server::setup_server};
use essential_types::contract::Contract;
use essential_types::PredicateAddress;
use token_app::{Addresses, App};

// Private key corresponding to the `MINTER` in the token contract
const MINTER_PRIV_KEY: &str = "128A3D2146A69581FD8FC4C0A9B7A96A5755D85255D4E47F814AFA69D7726C8D";

#[tokio::test]
async fn test_mint() {
    let (server_address, _server) = setup_server().await.unwrap();

    let mut wallet = essential_wallet::Wallet::temp().unwrap();

    // Setup an account for the deployer
    let deployer = "deployer".to_string();
    wallet
        .new_key_pair(&deployer, essential_wallet::Scheme::Secp256k1)
        .ok();

    // Set up an account for "alice" using the private key `MINTER_PRIV_KEY`
    let alice = "alice";
    wallet
        .insert_key(
            alice,
            essential_signer::Key::Secp256k1(
                essential_signer::secp256k1::SecretKey::from_slice(
                    &hex::decode(MINTER_PRIV_KEY).unwrap(),
                )
                .unwrap(),
            ),
        )
        .unwrap();

    // Set up another account for "bob"
    let bob = "bob";
    wallet
        .new_key_pair(bob, essential_wallet::Scheme::Secp256k1)
        .ok();

    // Compile the Pint contract and return the contract object
    let token_contract: Contract =
        compile_pint_project(concat!(env!("CARGO_MANIFEST_DIR"), "/../contract").into())
            .await
            .unwrap();

    // These are all the addresses we need addresses
    use essential_hash::{content_addr, contract_addr};
    let token_address = contract_addr::from_contract(&token_contract);
    let addresses = Addresses {
        token: token_address.clone(),
        mint: PredicateAddress {
            contract: token_address.clone(),
            predicate: content_addr(&token_contract.predicates[0]),
        },
        transfer: PredicateAddress {
            contract: token_address,
            predicate: content_addr(&token_contract.predicates[1]),
        },
    };

    // Sign and deploy the token contract at `server_address`. The deployer is `deployer`.
    essential_deploy_contract::sign_and_deploy(
        server_address.clone(),
        &deployer,
        &mut wallet,
        token_contract,
    )
    .await
    .unwrap();

    // This is a new instance of the `token` app
    let mut token = App::new(server_address.clone(), addresses, wallet).unwrap();

    // Ensure that the initial balance of `alice` is 0
    let balance = token.balance(alice).await.unwrap();
    assert_eq!(balance, 0);

    // Submit a valid solution to `Mint` that mints `mint_amount_1` tokens to `alice`
    let mint_amount_1 = 100_000;
    token.mint(alice, mint_amount_1).await.unwrap();

    // Ensure that the new balance is equal to `mint_amount_1`
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    let balance = token.balance(alice).await.unwrap();
    assert_eq!(balance, mint_amount_1);

    // Submit a valid solution to `Mint` that mints `mint_amount_2` tokens to `alice`
    let mint_amount_2 = 200_000;
    token.mint(alice, mint_amount_2).await.unwrap();

    // Ensure that the new balance is equal to `mint_amount_1 + mint_amount_2`
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    let balance = token.balance(alice).await.unwrap();
    assert_eq!(balance, mint_amount_1 + mint_amount_2);

    // Submit a valid solution to `Transfer` that transfers `transfer_amount` tokens from `alice`
    // to `bob`
    let transfer_amount = 500;
    token.transfer(alice, bob, transfer_amount).await.unwrap();

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    assert_eq!(token.balance(alice).await.unwrap(), 100_000 + 200_000 - 500);
    assert_eq!(token.balance(bob).await.unwrap(), 500);
}
