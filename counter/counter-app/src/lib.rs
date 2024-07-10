use anyhow::bail;
use essential_rest_client::EssentialClient;
use essential_types::{
    solution::{Mutation, Solution, SolutionData},
    PredicateAddress, Word,
};

pint_abi::gen_from_file!("../contract/out/debug/counter-abi.json");

pub struct App {
    client: EssentialClient,
    predicate_address: PredicateAddress,
}

impl App {
    /// The storage key where the `counter` storage variable is stored. This can be obtained from
    /// the JSON ABI
    pub const COUNTER_STORAGE_KEY: [Word; 1] = [0];

    /// Creates a new instance of the `counter` application. Takes a server address and a predicate
    /// address
    pub fn new(
        server_address: String,
        predicate_address: PredicateAddress,
    ) -> anyhow::Result<Self> {
        let client = EssentialClient::new(server_address)?;
        Ok(Self {
            client,
            predicate_address,
        })
    }

    /// Queries the state of the client for the current value of the counter, given the address of
    /// the contract that owns the counter and its storage key.
    pub async fn read_current_counter(&self) -> anyhow::Result<Word> {
        // Find the actualy storage key using the ABI
        let mut mutations: Vec<Mutation> = storage::mutations().counter(Default::default()).into();
        let key = mutations.pop().unwrap().key;

        let counter_value = self
            .client
            .query_state(&self.predicate_address.contract, &key)
            .await?;

        Ok(match &counter_value[..] {
            [] => 0, // Return 0 if `counter` has never been set before
            [count] => *count,
            _ => bail!("Expected one word, got: {:?}", counter_value),
        })
    }

    /// Increments the counter by crafting a solution and submitting it to the client
    pub async fn increment(&self) -> anyhow::Result<Word> {
        let new_count = self.read_current_counter().await? + 1;
        let solution = create_solution(self.predicate_address.clone(), new_count);
        self.client.submit_solution(solution).await?;
        Ok(new_count)
    }
}

/// Craft a solution that proposes a "state mutation" that sets the storage key of the counter to
/// `new_count`
pub fn create_solution(predicate_address: PredicateAddress, new_count: Word) -> Solution {
    Solution {
        data: vec![SolutionData {
            predicate_to_solve: predicate_address,
            decision_variables: Default::default(),
            transient_data: Default::default(),
            state_mutations: storage::mutations().counter(new_count).into(),
        }],
    }
}
