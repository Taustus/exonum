// Copyright 2020 The Exonum Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Cryptocurrency database schema.

use exonum::{
    crypto::Hash,
    merkledb::{
        access::{Access, FromAccess, RawAccessMut},
        Group, ObjectHash, ProofListIndex, RawProofMapIndex,
    },
    runtime::CallerAddress as Address,
};
use exonum_derive::{FromAccess, RequireArtifact};

use crate::{wallet::Wallet, INITIAL_BALANCE};
use crate::transactions::TxSendApprove;

/// Database schema for the cryptocurrency.
///
/// Note that the schema is crate-private, but it has a public part.
#[derive(Debug, FromAccess)]
pub(crate) struct SchemaImpl<T: Access> {
    /// Public part of the schema.
    #[from_access(flatten)]
    pub public: Schema<T>,
    /// History for specific wallets.
    pub wallet_history: Group<T, Address, ProofListIndex<T::Base, Hash>>,
}

/// Public part of the cryptocurrency schema.
#[derive(Debug, FromAccess, RequireArtifact)]
#[require_artifact(name = "exonum-cryptocurrency")]
pub struct Schema<T: Access> {
    /// Map of wallet keys to information about the corresponding account.
    pub wallets: RawProofMapIndex<T::Base, Address, Wallet>,
    /// Map of approval transactions hash to infromation about the corresponding approval transaction
    pub confirmed_transaction: RawProofMapIndex<T::Base, Hash, TxSendApprove>,
}

impl<T: Access> SchemaImpl<T> {
    pub fn new(access: T) -> Self {
        Self::from_root(access).unwrap()
    }

    pub fn wallet(&self, address: Address) -> Option<Wallet> {
        self.public.wallets.get(&address)
    }

    pub fn confirmed(&self, hash: Hash) -> Option<TxSendApprove> {
        self.public.confirmed_transaction.get(&hash)
    }
}

impl<T> SchemaImpl<T>
where
    T: Access,
    T::Base: RawAccessMut,
{
    pub fn create_send_approve_transaction(&mut self,
                                           wallet: Wallet,
                                           amount: u64,
                                           to: Address,
                                           approver: Address,
                                           transaction: Hash) {
        self.increase_frozen_balance(wallet,  amount as i64, transaction);
        self.public.confirmed_transaction.put(&transaction, TxSendApprove::new(to, amount, approver))
    }

    /// Increases frozen of the wallet and append new record to its history.
    pub fn increase_frozen_balance(&mut self,
                                 wallet: Wallet,
                                 frozen_balance_change: i64,
                                 transaction: Hash) {
        let mut history = self.wallet_history.get(&wallet.owner);
        history.push(transaction);
        let history_hash = history.object_hash();

        let wallet_frozen_balance = (wallet.frozen_balance as i64);
        let wallet = wallet.set_frozen_balance(( wallet_frozen_balance + frozen_balance_change) as u64, &history_hash);

        let wallet_key = wallet.owner;
        self.public.wallets.put(&wallet_key, wallet);
    }

    /// Decreases frozen of the wallet and append new record to its history.
    pub fn decrease_frozen_balance(&mut self,
                                   wallet: Wallet,
                                   frozen_balance_change: u64,
                                   transaction: Hash) {
        let mut history = self.wallet_history.get(&wallet.owner);
        history.push(transaction);
        let history_hash = history.object_hash();

        let wallet_frozen_balance = wallet.frozen_balance;

        let dif_froz = wallet_frozen_balance - frozen_balance_change;
        let dif_bal = wallet.balance - frozen_balance_change;

        let wallet = wallet.set_frozen_balance(dif_froz, &history_hash);
        let wallet = wallet.set_balance(dif_bal, &history_hash);

        let wallet_key = wallet.owner;
        self.public.wallets.put(&wallet_key, wallet);
    }

    /// Increases balance of the wallet and append new record to its history.
    pub fn increase_wallet_balance(&mut self, wallet: Wallet, amount: u64, transaction: Hash) {
        let mut history = self.wallet_history.get(&wallet.owner);
        history.push(transaction);
        let history_hash = history.object_hash();
        let balance = wallet.balance;
        let wallet = wallet.set_balance(balance + amount, &history_hash);
        let wallet_key = wallet.owner;
        self.public.wallets.put(&wallet_key, wallet);
    }

    /// Decreases balance of the wallet and append new record to its history.
    pub fn decrease_wallet_balance(&mut self, wallet: Wallet, amount: u64, transaction: Hash) {
        let mut history = self.wallet_history.get(&wallet.owner);
        history.push(transaction);
        let history_hash = history.object_hash();
        let balance = wallet.balance;
        let wallet = wallet.set_balance(balance - amount, &history_hash);
        let wallet_key = wallet.owner;
        self.public.wallets.put(&wallet_key, wallet);
    }

    /// Creates a new wallet and append first record to its history.
    pub fn create_wallet(&mut self, key: Address, name: &str, transaction: Hash) {
        let mut history = self.wallet_history.get(&key);
        history.push(transaction);
        let history_hash = history.object_hash();
        let wallet = Wallet::new(key, name, INITIAL_BALANCE, 0, history.len(), &history_hash);
        self.public.wallets.put(&key, wallet);
    }
}
