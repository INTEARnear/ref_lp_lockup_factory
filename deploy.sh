CONTRACT_ID=$1
OWNER_ID=$2
COST_NEAR=$3
NETWORK=$4
CONTRACT_FILE=$5
CREATE=$6

cargo build --target wasm32-unknown-unknown --release

if [ "$CREATE" = "true" ]; then
  near account create-account \
    sponsor-by-faucet-service \
    "$CONTRACT_ID" \
    autogenerate-new-keypair \
    save-to-keychain \
    network-config "$NETWORK" \
    create
fi

near contract deploy "$CONTRACT_ID" \
  use-file ./target/wasm32-unknown-unknown/release/ref_lp_lockup_factory.wasm \
  with-init-call new \
  json-args "{\"owner\":\"$OWNER_ID\",\"register_cost\":\"${COST_NEAR}000000000000000000000000\"}" \
  prepaid-gas '300.0 Tgas' \
  attached-deposit '0 NEAR' \
  network-config "$NETWORK" \
  sign-with-keychain send

near contract call-function as-transaction "$CONTRACT_ID" \
  update_stored_contract file-args "$CONTRACT_FILE" \
  prepaid-gas '300.0 Tgas' \
  attached-deposit '0 NEAR' \
  sign-as "$CONTRACT_ID" \
  network-config "$NETWORK" \
  sign-with-keychain send
