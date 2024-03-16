use near_sdk::{AccountId, env, near, NearToken, PanicOnDefault, Promise, log, require, Gas, PromiseError};
use near_sdk::json_types::U64;
use near_sdk::store::LazyOption;

const NEAR_PER_STORAGE: NearToken = NearToken::from_yoctonear(10_000_000_000_000_000_000); // 10e18yⓃ

#[derive(PanicOnDefault)]
#[near(contract_state)]
pub struct LockupFactory {
    owner: AccountId,
    register_cost: NearToken,
    lockup_contract_code: LazyOption<Vec<u8>>,
}

#[near(serializers = [json])]
struct LockupInitParams {
    pool_id: U64,
    ref_address: AccountId,
}

#[near]
impl LockupFactory {
    #[init]
    pub fn new(owner: AccountId, register_cost: NearToken) -> Self {
        Self {
            owner,
            register_cost,
            lockup_contract_code: LazyOption::new("code".as_bytes().to_vec(), None),
        }
    }

    pub fn get_register_cost(&self) -> NearToken {
        self.register_cost
    }

    pub fn set_register_cost(&mut self, register_cost: NearToken) {
        self.assert_owner();
        self.register_cost = register_cost;
    }

    fn assert_owner(&self) {
        require!(env::predecessor_account_id() == self.owner, "Only owner can call this method");
    }

    #[private]
    pub fn update_stored_contract(&mut self) {
        self.lockup_contract_code.set(Some(env::input().expect("No input").to_vec()));
        log!("Contract code updated");
    }

    #[payable]
    pub fn register_pool(&mut self, pool_id: U64, ref_address: AccountId) {
        let attached = env::attached_deposit();
        require!(attached == self.register_cost, format!("The amount sent is not the expected amount ({})", self.register_cost));

        require!(self.lockup_contract_code.is_some(), "Lockup contract is not set");
        let code = self.lockup_contract_code.as_ref().unwrap();
        let contract_bytes = code.len() as u128;
        let minimum_needed = NEAR_PER_STORAGE.as_yoctonear() * (contract_bytes + 5 * 1024);
        require!(
            attached.as_yoctonear() >= minimum_needed,
            "Register cost is lower than storage cost (this should never happen)"
        );

        let current_account = env::current_account_id().to_string();
        let subaccount: AccountId = format!("{}.{}", pool_id.0, current_account).parse().unwrap();
        require!(
            env::is_valid_account_id(subaccount.as_bytes()),
            "Invalid subaccount id"
        );
        log!("Locking pool {}", pool_id.0);
        Promise::new(subaccount.clone())
            .create_account()
            .transfer(NearToken::from_yoctonear(minimum_needed))
            .deploy_contract(code.clone())
            .function_call(
                "new".to_string(),
                near_sdk::serde_json::to_vec(&LockupInitParams { pool_id, ref_address }).unwrap(),
                NearToken::from_yoctonear(0),
                Gas::from_tgas(50),
            )
            .then(Self::ext(env::current_account_id())
                .with_static_gas(Gas::from_tgas(10))
                .register_pool_callback(env::attached_deposit(), NearToken::from_yoctonear(minimum_needed)));
    }

    #[private]
    pub fn register_pool_callback(&mut self, attached: NearToken, minimum_needed: NearToken, #[callback_result] call_result: Result<(), PromiseError>) {
        if call_result.is_err() {
            log!("Error registering pool");
            // No refunds if you try to overclock
        } else {
            let transferable_amount = NearToken::from_yoctonear(attached.as_yoctonear() - minimum_needed.as_yoctonear());
            Promise::new(self.owner.clone()).transfer(transferable_amount);
        }
    }
}
