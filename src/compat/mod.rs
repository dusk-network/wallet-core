// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

/// Helping us with the crypto primitives
pub mod crypto;
/// Includes methods to deal with bip39::Mnemonic
pub mod mnemonic;
/// Includes functions to rkyv serialize types like phoenix_core and crypto
/// primitives
pub mod rkyv;
/// Includes functions to interact with the stake contract
pub mod stake;
/// Includes functions to deal with UnprovenTransaction and Transaction
pub mod tx;
/// Includes functions to interact with the stake contract unstake tx
pub mod unstake;
/// Includes functions to interact with the stake contract withdraw tx
pub mod withdraw;

mod stake_contract_types {
    pub use stake_contract_types::{
        allow_signature_message, stake_signature_message,
        unstake_signature_message, withdraw_signature_message,
    };
    pub use stake_contract_types::{Allow, Stake, Unstake, Withdraw};
}
