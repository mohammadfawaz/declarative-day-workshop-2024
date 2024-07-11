use anyhow::bail;
use essential_rest_client::EssentialClient;
use essential_types::{
    convert::word_4_from_u8_32,
    solution::{Mutation, Solution, SolutionData},
    ContentAddress, PredicateAddress, Word,
};
use essential_wallet::Wallet;

pint_abi::gen_from_file!("../contract/out/debug/token-abi.json");

#[derive(Debug, Clone)]
pub struct Addresses {
    pub token: ContentAddress,
    pub mint: PredicateAddress,
    pub transfer: PredicateAddress,
}

pub struct App {
    client: EssentialClient,
    wallet: Wallet,
    addresses: Addresses,
}

/// Core Methods
impl App {
    /// Creates a new instance of the `token` application. Takes a server address, the predicate
    /// addresses, and a wallet
    pub fn new(
        server_address: String,
        addresses: Addresses,
        wallet: essential_wallet::Wallet,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            client: EssentialClient::new(server_address)?,
            addresses,
            wallet,
        })
    }

    /// Crafts a solution to the `Mint` predicate. Mints `amount` tokens to `to_name`.
    pub async fn mint(&mut self, to_name: &str, amount: Word) -> anyhow::Result<()> {
        // The key which will receive the minted tokens
        let to = self.get_hashed_key(to_name)?;

        // This is the current balace of `to_name`. We later add `amount` to this and use it
        // as the new balance in the solution.
        let current_balance = self.balance(to_name).await?;

        // Produce a signature over `to` and `amount`. `sign_data` expects a flattened vector of
        // words of all the data to sign.
        let mut data_to_sign = to.to_vec();
        data_to_sign.push(amount);
        let signature = self.sign_data(to_name, data_to_sign)?;

        // Craft and submit the solution
        let solution = Solution {
            data: vec![SolutionData {
                predicate_to_solve: self.addresses.mint.clone(),
                decision_variables: Mint::Vars {
                    to,
                    amount,
                    signature,
                }
                .into(),
                transient_data: Default::default(),
                state_mutations: storage::mutations()
                    .balances(|map| map.entry(to, current_balance + amount))
                    .into(),
            }],
        };

        self.client.submit_solution(solution).await?;
        Ok(())
    }

    /// Crafts a solution to the `Transfer` predicate. Transfers `amount` tokens from `from_name`
    /// to `to_name`.
    pub async fn transfer(
        &mut self,
        from_name: &str,
        to_name: &str,
        amount: Word,
    ) -> anyhow::Result<()> {
        // The key which will send the tokens
        let from = self.get_hashed_key(from_name)?;

        // The key which will receive the tokens
        let to = self.get_hashed_key(to_name)?;

        let current_from_balance = self.balance(from_name).await?;
        let current_to_balance = self.balance(to_name).await?;

        // Produce a signature over `from`, `to`, and `amount`. `sign_data` expects a flattened
        // vector of words of all the data to sign.
        let mut data_to_sign = from.to_vec();
        data_to_sign.extend(to);
        data_to_sign.push(amount);
        let signature = self.sign_data(from_name, data_to_sign)?;

        // Craft and submit the solution
        let solution = Solution {
            data: vec![SolutionData {
                predicate_to_solve: self.addresses.transfer.clone(),
                decision_variables: Transfer::Vars {
                    from,
                    to,
                    amount,
                    signature,
                }
                .into(),
                transient_data: Default::default(),
                state_mutations: storage::mutations()
                    .balances(|map| map.entry(from, current_from_balance - amount))
                    .balances(|map| map.entry(to, current_to_balance + amount))
                    .into(),
            }],
        };

        self.client.submit_solution(solution).await?;
        Ok(())
    }
}

/// Utility Methods
impl App {
    /// Query the client to find the balance of `account_name`
    pub async fn balance(&mut self, account_name: &str) -> anyhow::Result<Word> {
        let account_key = self.get_hashed_key(account_name)?;

        // Find the actual storage key using the ABI
        let mut mutations: Vec<Mutation> = storage::mutations()
            .balances(|map| map.entry(account_key, Default::default()))
            .into();
        let key = mutations.pop().unwrap().key;

        // Query the client to read the value at `key`
        let balance_value = self.client.query_state(&self.addresses.token, &key).await?;

        Ok(match &balance_value[..] {
            [] => 0, // Return 0 if the balance of `account_name` has never been set before
            [balance] => *balance,
            _ => bail!("Expected one word, got: {:?}", balance_value),
        })
    }

    /// Given an account name, produce the hash of its public key. The result is what we use in the
    /// contract to refer to accounts (e.g. in the `balances` storage map)
    fn get_hashed_key(&mut self, account_name: &str) -> anyhow::Result<[Word; 4]> {
        let public_key = self.wallet.get_public_key(account_name)?;
        let essential_signer::PublicKey::Secp256k1(public_key) = public_key else {
            anyhow::bail!("Invalid public key")
        };
        let encoded = essential_sign::encode::public_key(&public_key);
        Ok(word_4_from_u8_32(essential_hash::hash_words(&encoded)))
    }

    fn sign_data(
        &mut self,
        account_name: &str,
        data: Vec<Word>,
    ) -> anyhow::Result<([Word; 4], [Word; 4], Word)> {
        let sig = self.wallet.sign_words(&data, account_name)?;
        let sig = match sig {
            essential_signer::Signature::Secp256k1(sig) => sig,
            _ => bail!("Invalid signature"),
        };

        let [a0, a1, a2, a3, b0, b1, b2, b3, c1] = essential_sign::encode::signature(&sig);

        Ok(([a0, a1, a2, a3], [b0, b1, b2, b3], c1))
    }
}
