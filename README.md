# Regen-Contract

This is a CosmWasm contract for the regen testnet. It is written in Rust and compiles to Wasm. It is based on the escrow example provided by the CosmWasm team at https://github.com/CosmWasm/cosmwasm-examples/tree/master/escrow.

For more details about the contract you can read https://github.com/regen-network/testnets/blob/master/kontraua/challenges/phase-5/README.md .


## Build And Deploy

This guide helps you to deploy the contract.

Note: This document borrows most of the instructions from [COSMWASM official docs](https://www.cosmwasm.com/docs/getting-started/intro), thanks to **Ethan Frey** and team.

### Pre-requisites

#### Install Rust

Please feel free to refer [rust basics](https://www.cosmwasm.com/docs/getting-started/rust-basics) for more details.

`rustup` is an installer for the systems programming language [Rust](https://www.rust-lang.org/)

Run the following in your terminal, then follow the onscreen instructions.

```
$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Once installed, make sure you have the wasm32 target, both on stable and nightly:
```
$ rustup default stable
$ rustup target list --installed
$ rustup target add wasm32-unknown-unknown

$ rustup install nightly
$ rustup target add wasm32-unknown-unknown --toolchain nightly
```

### Download and edit/update the contract

#### Get the code

```
$ git clone https://github.com/BlockscapeLab/regen_contract
$ cd regen_contract
```

#### Compile the wasm contract with stable toolchain
```
rustup default stable
cargo wasm
```
After this compiles, it should produce a file at  `target/wasm32-unknown-unknown/release/escrow.wasm`. A quick `ls -l` should show around 1.5MB. This is a release build, but not stripped of all unneeded code

#### Compiling for Production
You can check the size of the contract file by running:
```
$ du -h target/wasm32-unknown-unknown/release/cw_escrow.wasm

// Outputs
1.9M    target/wasm32-unknown-unknown/release/cw_escrow.wasm
```
This works, but is huge for a blockchain transaction. Let's try to make it smaller. Turns out there is a linker flag to strip off debug information:

```
$ RUSTFLAGS='-C link-arg=-s' cargo wasm
$ du -h target/wasm32-unknown-unknown/release/cw_escrow.wasm

// Outputs
128K    target/wasm32-unknown-unknown/release/cw_escrow.wasm
```
This is looking much better in size.

Those who wants to experiment with other cool features can try out: [reproduceable builds](https://www.cosmwasm.com/docs/getting-started/editing-escrow-contract#reproduceable-builds)

### Deploy your contract

All the wasm related commands can be found at:
```
xrncli tx wasm -h
```

And, related queries at: 
```
xrncli query wasm -h
```

#### Step - 1 : Upload your contract

To upload a new contract,

```
xrncli tx wasm store contract.wasm --gas auto --from <key_name> --node <rpc_endpoint> -y

#Add keys for arbiter, recipient

xrncli keys add fred
xrncli keys add bob
```

You can check your code id by querying the upload transaction:

```
xrncli query tx <tx hash> --trust-node --node <rpc-address> --output json
```

For more details about uploading contract, check the details here: https://www.cosmwasm.com/docs/getting-started/first-demo#uploading-the-code

#### Step - 2: Instantiating the Contract


```
# Please make sure to add keys of bob and fred to your keyring. Also Accounts of Fred and Bob must contain any coins to be used.
# Insert an appropriate end_height in the INIT msg to make sure the escrow has am expiry height.

INIT="{\"region\":\"Germany\",\"beneficiary\":\"$(xrncli keys show bob -a)\",\"oracle\":\"$(xrncli keys show fred -a)\",\"ecostate\":2000,\"total_tokens\":5000,\"payout_start_height\":0,\"payout_end_height\":100000000000}"

# Please Note: amount must match total tokens from above. These are the tokens the contract owns and can pay out.
xrncli tx wasm instantiate <code_id> "$INIT" --from <key_name> --label "payout 1 <moniker>" --node <rpc_endpoint> --chain-id kontraua --amount 5000utree -y 
```

Verify your code instance:
```
# check the contract state (and account balance)
xrncli query wasm list-contract-by-code <code_id>  --node <rpc_endpoint> --chain-id kontraua -o json

# contracts ids (like code ids) are based on an auto-gen sequence
# if this is the first contract in the devnet, it will have this address (otherwise, use the result from list-contract-by-code)
CONTRACT=xrn:10pyejy66429refv3g35g2t7am0was7ya75d7y2

# query contract to verify init message
xrncli query wasm contract $CONTRACT --node <rpc_endpoint> --chain-id kontraua --trust-node -o json

# you can query contract address as normal account
xrncli query account $CONTRACT --node <rpc_endpoint> --chain-id kontraua --trust-node -o json

# you can dump entire contract state
xrncli query wasm contract-state all $CONTRACT

# Note that keys are hex encoded, and val is base64 encoded.
# To view the returned data (assuming it is ascii), try something like:
# (Note that in many cases the binary data returned is non in ascii format, thus the encoding)
xrncli query wasm contract-state all $CONTRACT | jq -r .[0].key | xxd -r -ps
xrncli query wasm contract-state all $CONTRACT | jq -r .[0].val | base64 -d

```

#### Step-3: Execute contract functions

##### Messages


Lock: "{\"lock\": {}}"
Description: Locks the contract. No more payouts possible until unlocked. Can be only executed by contract owner.

Unlock: "{\"unlock\": {}}"
Description: Unlocks the contract. Payouts are possible again. Can be only executed by contract owner.


Transfer Ownership: "{\"transferownership\":{\"owner\":\"xrn:1x3rlgy9sjtp7p29c49s0avs2ln5a9k5yf2zzvs\"}}"
Description: Transfers ownership to new account. Can only executed by current owner.

Change Beneficiary: "{\"changebeneficiary\":{\"beneficiary\": \"xrn:10l9t2lvxhxut2s60lwct8f6n2q5x88khe7ytdg\"}}"
Description: Changes the beneficiary. Can only be executed by the current beneficiary.

Payout: "{\"updateecostate\":{\"ecostate\":2347}}"
Description: Updates the ecostate of the region. A value of 2347 means new ecostate is 23,47%. Based on the improvements coins are paid out to the beneficiary. Can only be executed by the oracle. 

##### Execute Messages

Set the msg:
```
# e.g.:  "{\"unlock\": {}}"
MSG=<msg>  
```

Execute the approve function 
```
xrncli tx wasm execute $CONTRACT "$MSG" --from <account> -y --chain-id kontraua

```