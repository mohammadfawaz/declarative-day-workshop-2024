use anyhow::bail;
use essential_rest_client::EssentialClient;
use essential_types::{
    solution::{Mutation, Solution, SolutionData},
    PredicateAddress, Word,
};

pint_abi::gen_from_file!("../contract/out/debug/counter-abi.json");

pub struct App {
    client: EssentialClient,
    increment_predicate_address: PredicateAddress,
}

/// Core Methods
impl App {
    /// Creates a new instance of the `counter` application. Takes a server address and a predicate
    /// address
    pub fn new(
        server_address: String,
        increment_predicate_address: PredicateAddress,
    ) -> anyhow::Result<Self> {
        let client = EssentialClient::new(server_address)?;
        Ok(Self {
            client,
            increment_predicate_address,
        })
    }

    /// Increments the counter by crafting a solution and submitting it to the client
    pub async fn increment(&self) -> anyhow::Result<Word> {
        let new_count = self.read_current_counter().await? + 1;
        let solution = Solution {
            data: vec![SolutionData {
                predicate_to_solve: self.increment_predicate_address.clone(),
                decision_variables: Default::default(),
                transient_data: Default::default(),
                state_mutations: storage::mutations().counter(new_count).into(),
            }],
        };
        self.client.submit_solution(solution).await?;
        Ok(new_count)
    }
}

/// Utility Methods
impl App {
    /// Queries the state of the client for the current value of the counter, given the address of
    /// the contract that owns the counter and its storage key.
    pub async fn read_current_counter(&self) -> anyhow::Result<Word> {
        // Find the actual storage key using the ABI
        let mut mutations: Vec<Mutation> = storage::mutations().counter(Default::default()).into();
        let key = mutations.pop().unwrap().key;

        let counter_value = self
            .client
            .query_state(&self.increment_predicate_address.contract, &key)
            .await?;

        Ok(match &counter_value[..] {
            [] => 0, // Return 0 if `counter` has never been set before
            [count] => *count,
            _ => bail!("Expected one word, got: {:?}", counter_value),
        })
    }
}
