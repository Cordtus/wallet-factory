use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Wallet {
    pub address: String,  // Bech32 address
    #[serde(rename = "evmAddress", skip_serializing_if = "Option::is_none")]
    pub evm_address: Option<String>,  // EVM address
    pub pubkey: String,
    #[serde(rename = "privateKey")]
    pub private_key: String,
    #[serde(rename = "derivationPath")]
    pub derivation_path: String,
}