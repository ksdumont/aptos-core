---
title: "Transactions and States"
slug: "txns-states"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Transactions and States

The two fundamental concepts at the heart of the Aptos blockchain are transactions and states:

* **Transactions**: Transactions represent the exchange of data (e.g., Aptos Coins or NFTs) between accounts on the Aptos blockchain.
* **States**: The state, i.e., the Aptos blockchain ledger state, represents the state of all the accounts in the Aptos blockchain. 

:::tip
Only executing transactions can change the ledger state.
:::

## Transactions

Aptos transactions contain information such as the sender’s account address, authentication from the sender, the desired operation to be performed on the Aptos blockchain, and the amount of gas the sender is willing to pay to execute the transaction.

### Transaction states

A transaction may end in one of the following states:

* Committed on the blockchain and executed. This is considered as a successful transaction.
* Committed on the blockchain and aborted. The abort code indicates why the transaction failed to execute.
* Discarded during transaction submission due to a validation check such as insufficient gas, invalid transaction format, or incorrect key.
* Discarded after transaction submission but before attempted execution. This could be caused by timeouts or insufficient gas due to other transactions affecting the account.

The sender’s account will be charged gas for any committed transactions.

During transaction submission, the submitter is notified of successful submission or a reason for failing validations otherwise.

A transaction that is successfully submitted but ultimately discarded may have no visible state in any accessible Aptos node or within the Aptos network. A user can attempt to resubmit the same transaction to re-validate the transaction. If the submitting node believes that this transaction is still valid, it will return an error stating that an identical transaction has been submitted.

The submitter can try to increase the gas cost by a trivial amount to help make progress and adjust for whatever may have been causing the discarding of the transaction further downstream.

:::tip Read more
See [Aptos Blockchain Deep Dive](./blockchain.md) for a comprehensive description of the Aptos transaction lifecycle.
:::

### Contents of a Transaction

A signed transaction on the blockchain contains the following information:

- **Signature**: The sender uses a digital signature to verify that they signed the transaction (i.e., authentication).
- **Sender address**: The sender's [account address](./accounts.md#account-address).
- **Sender public key**: The public authentication key that corresponds to the private authentication key used to sign the transaction.
- **Payload**: Indicates an action or set of actions Alice's behalf. In the case this is a Move function, it directly calls into Move bytecode on the chain. Alternatively, it may be Move bytecode peer-to-peer [transaction script](../reference/glossary.md#transaction-script). It also contains a list of inputs to the function or script. For this example, it is a function call to transfer an amount of Aptos Coins from Alice account to Bob's account, where Alice's account is implied by sending the transaction and Bob's account and the amount are specified as transaction inputs.
- [**Gas unit price**](../reference/glossary.md#gas-unit-price): The amount the sender is willing to pay per unit of gas, to execute the transaction. This is represented as Octa or units of 10<sup>-8</sup> utility tokens.
- [**Maximum gas amount**](../reference/glossary.md#maximum-gas-amount): The maximum gas amount in Aptos utility tokens the sender is willing to pay for this transaction. Gas charges are equal to the base gas cost covered by computation and IO multiplied by the gas price. Gas costs also include storage with an Apt-fixed priced storage model. This is represents as Octa or units of 10<sup>-8</sup> Aptos utility tokens.
- **Gas price** (in specified gas units): This is the amount the sender is willing to pay per unit of [gas](./gas-txn-fee.md) to execute the transaction. [Gas](./gas-txn-fee.md) is a way to pay for computation and storage. A gas unit is an abstract measurement of computation with no inherent real-world value.
- **Maximum gas amount**: The [maximum gas amount](./gas-txn-fee.md#gas-and-transaction-fee-on-the-aptos-blockchain) is the maximum gas units the transaction is allowed to consume.
- **Sequence number**: This is an unsigned integer that must be equal to the sender's account [sequence number](./accounts.md#account-sequence-number) at the time of execution.
- **Expiration time**: A timestamp after which the transaction ceases to be valid (i.e., expires).

### Types of transaction payloads
Within a given transaction, the two most common types of payloads include:

- An entry point
- [A script (payload)](../move/move-on-aptos/move-scripts)

Currently the SDKs [Python](https://aptos.dev/sdks/python-sdk) and [Typescript](https://aptos.dev/sdks/ts-sdk/index) support both. This guide points out many of those entry points, such as `coin::transfer` and `aptos_account::create_account`.

All operations on the Aptos blockchain should be available via entry point calls. While one could submit multiple transactions calling entry points in series, many such operations may benefit from being called atomically from a single transaction. A script payload transaction can call any entry point or public function defined within any module.

:::tip Read more
See the tutorial on [Your First Transaction](../tutorials/first-transaction.md) for generating valid transactions.
:::

:::note Transaction generation
The Aptos REST API supports generating BCS-encoded transactions from JSON. This is useful for rapid prototyping, but be cautious using it in Mainnet as this places a lot of trust on the fullnode generating the transaction.
:::

## States

The Aptos blockchain's ledger state, or global state, represents the state of all accounts in the Aptos blockchain. Each validator node in the blockchain must know the latest version of the global state to execute any transaction.

Anyone can submit a transaction to the Aptos blockchain to modify the ledger state. Upon execution of a transaction, a transaction output is generated. A transaction output contains zero or more operations to manipulate the ledger state called **write sets** emitting a vector of resulting events, the amount of gas consumed, and the executed transaction status.

### Proofs

The Aptos blockchain uses proof to verify the authenticity and correctness of the blockchain data.

Data within the Aptos blockchain is replicated across the network. Each validator and fullnode's [storage](./validator-nodes#storage) is responsible for persisting the agreed upon blocks of transactions and their execution results to the database. 

The blockchain is represented as an ever-growing [Merkle tree](../reference/glossary.md#merkle-trees), where each leaf appended to the tree represents a single transaction executed by the blockchain.

All operations executed by the blockchain and all account states can be verified cryptographically. These cryptographic proofs ensure that:
- The validator nodes agree on the state. 
- The client does not need to trust the entity from which it is receiving data. For example, if a client fetches the last **n** transactions from an account, a proof can attest that no transactions were added, omitted or modified in the response. The client may also query for the state of an account, ask whether a specific transaction was processed, and so on.

### Versioned database

The ledger state is versioned using an unsigned 64-bit integer corresponding to the number of transactions the system has executed. This versioned database allows the validator nodes to:

- Execute a transaction against the ledger state at the latest version.
- Respond to client queries about ledger history at both current and previous versions.

## Transactions change ledger state

<ThemedImage
alt="Signed Transaction Flow"
sources={{
    light: useBaseUrl('/img/docs/transactions-and-state.svg'),
    dark: useBaseUrl('/img/docs/transactions-and-state-dark.svg'),
  }}
/>

The above figure shows how executing transaction T<sub>*i*</sub> changes the state of the Aptos blockchain from S<sub>*i-1*</sub> to S<sub>*i*</sub>.

In the figure:

- Accounts **A** and **B**: Represent Alice's and Bob's accounts on the Aptos blockchain.
- **S<sub>*i-1*</sub>** : Represents the (*i-1*)-the state of the blockchain. In this state, Alice's account **A** has a balance of 110 APT (Aptos coins), and Bob's account **B** has a balance of 52 APT.
- **T<sub>*i*</sub>** : This is the *i*-th transaction executed on the blockchain. In this example, it represents Alice sending 10 APT to Bob.
- **Apply()**: This is a deterministic function that always returns the same final state for a specific initial state and a specific transaction. If the current state of the blockchain is **S<sub>*i-1*</sub>**, and transaction **T<sub>*i*</sub>** is executed on the state **S<sub>*i-1*</sub>**, then the new state of the blockchain is always **S<sub>*i*</sub>**. The Aptos blockchain uses the [Move language](../move/book/SUMMARY.md) to implement the deterministic execution function **Apply()**.
- **S<sub>*i*</sub>** : This is the *i*-the state of the blockchain. When the transaction **T<sub>*i*</sub>** is applied to the blockchain, it generates the new state **S<sub>*i*</sub>** (an outcome of applying **Apply(S<sub>*i-1*</sub>, T<sub>*i*</sub>)** to **S<sub>*i-1*</sub>** and **T<sub>*i*</sub>**). This causes Alice’s account balance to be reduced by 10 to 100 APT and Bob’s account balance to be increased by 10 to 62 APT. The new state **S<sub>*i*</sub>** shows these updated balances.
