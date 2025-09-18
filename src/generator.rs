use anyhow::Result;
use base64::{Engine as _, engine::general_purpose};
use bech32::{Bech32, Hrp};
use hex;
use secp256k1::{Secp256k1, SecretKey, PublicKey};
use sha2::{Sha256, Digest};
use sha3::Keccak256;
use ripemd::Ripemd160;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tiny_hderive::bip32::ExtendedPrivKey;

use crate::wallet::Wallet;
use crate::cli::KeyType;

// Pre-compute and cache the secp256k1 context
thread_local! {
    static SECP: Secp256k1<secp256k1::All> = Secp256k1::new();
}

#[inline(always)]
pub fn generate_addresses(private_key: &[u8; 32], prefix: &str, key_type: &KeyType) -> Result<(String, Option<String>, String, String)> {
    SECP.with(|secp| {
        // Create secret key from bytes
        let secret_key = SecretKey::from_slice(private_key)?;

        // Generate public key
        let public_key = PublicKey::from_secret_key(secp, &secret_key);

        // Convert private key to hex once (used by both paths)
        let private_key_hex = hex::encode(private_key);

        match key_type {
            KeyType::Secp256k1 => {
                // Standard secp256k1 path
                let pubkey_compressed = public_key.serialize();
                let pubkey_base64 = general_purpose::STANDARD.encode(&pubkey_compressed);

                // SHA256 -> RIPEMD160
                let sha256_hash = Sha256::digest(&pubkey_compressed);
                let ripemd_hash = Ripemd160::digest(&sha256_hash);

                let hrp = Hrp::parse(prefix)?;
                let cosmos_addr = bech32::encode::<Bech32>(hrp, &ripemd_hash[..])?;

                Ok((cosmos_addr, None, pubkey_base64, private_key_hex))
            },
            KeyType::Ethsecp256k1 => {
                // Ethereum-compatible path
                let pubkey_compressed = public_key.serialize();
                let pubkey_base64 = general_purpose::STANDARD.encode(&pubkey_compressed);

                // Keccak256 of uncompressed pubkey
                let pubkey_uncompressed = public_key.serialize_uncompressed();
                let keccak_hash = Keccak256::digest(&pubkey_uncompressed[1..]); // Skip 0x04
                let address_bytes = &keccak_hash[12..];

                let hrp = Hrp::parse(prefix)?;
                let cosmos_addr = bech32::encode::<Bech32>(hrp, address_bytes)?;
                let evm_addr = format!("0x{}", hex::encode(address_bytes));

                Ok((cosmos_addr, Some(evm_addr), pubkey_base64, private_key_hex))
            }
        }
    })
}

pub fn generate_wallets_batch(
    seed: &[u8],
    start_index: usize,
    count: usize,
    prefix: &str,
    key_type: &KeyType,
    progress: Arc<AtomicUsize>,
) -> Vec<Wallet> {
    match key_type {
        KeyType::Secp256k1 => generate_secp256k1_batch(seed, start_index, count, prefix, progress),
        KeyType::Ethsecp256k1 => generate_ethsecp256k1_batch(seed, start_index, count, prefix, progress),
    }
}

#[inline]
fn generate_secp256k1_batch(
    seed: &[u8],
    start_index: usize,
    count: usize,
    prefix: &str,
    progress: Arc<AtomicUsize>,
) -> Vec<Wallet> {
    let mut wallets = Vec::with_capacity(count);
    let hrp = Hrp::parse(prefix).expect("Invalid prefix");

    SECP.with(|secp| {
        for i in 0..count {
            let index = start_index + i;
            let path = format!("m/44'/118'/0'/0/{}", index);

            if let Ok(derived_key) = ExtendedPrivKey::derive(seed, path.as_str()) {
                let private_key = derived_key.secret();
                let secret_key = SecretKey::from_slice(&private_key).expect("Invalid key");
                let public_key = PublicKey::from_secret_key(secp, &secret_key);

                // Standard secp256k1
                let pubkey_compressed = public_key.serialize();
                let pubkey_base64 = general_purpose::STANDARD.encode(&pubkey_compressed);
                let private_key_hex = hex::encode(&private_key);

                // SHA256 -> RIPEMD160
                let sha256_hash = Sha256::digest(&pubkey_compressed);
                let ripemd_hash = Ripemd160::digest(&sha256_hash);

                if let Ok(cosmos_addr) = bech32::encode::<Bech32>(hrp.clone(), &ripemd_hash[..]) {
                    wallets.push(Wallet {
                        address: cosmos_addr,
                        evm_address: None,
                        pubkey: pubkey_base64,
                        private_key: private_key_hex,
                        derivation_path: path,
                    });
                }
            }

            if i % 1000 == 0 {
                progress.fetch_add(1000, Ordering::Relaxed);
            }
        }
    });

    progress.fetch_add(count % 1000, Ordering::Relaxed);
    wallets
}

#[inline]
fn generate_ethsecp256k1_batch(
    seed: &[u8],
    start_index: usize,
    count: usize,
    prefix: &str,
    progress: Arc<AtomicUsize>,
) -> Vec<Wallet> {
    let mut wallets = Vec::with_capacity(count);
    let hrp = Hrp::parse(prefix).expect("Invalid prefix");

    SECP.with(|secp| {
        for i in 0..count {
            let index = start_index + i;
            let path = format!("m/44'/118'/0'/0/{}", index);

            if let Ok(derived_key) = ExtendedPrivKey::derive(seed, path.as_str()) {
                let private_key = derived_key.secret();
                let secret_key = SecretKey::from_slice(&private_key).expect("Invalid key");
                let public_key = PublicKey::from_secret_key(secp, &secret_key);

                // ethsecp256k1
                let pubkey_compressed = public_key.serialize();
                let pubkey_base64 = general_purpose::STANDARD.encode(&pubkey_compressed);
                let private_key_hex = hex::encode(&private_key);

                // Keccak256
                let pubkey_uncompressed = public_key.serialize_uncompressed();
                let keccak_hash = Keccak256::digest(&pubkey_uncompressed[1..]);
                let address_bytes = &keccak_hash[12..];

                if let Ok(cosmos_addr) = bech32::encode::<Bech32>(hrp.clone(), address_bytes) {
                    let evm_addr = format!("0x{}", hex::encode(address_bytes));

                    wallets.push(Wallet {
                        address: cosmos_addr,
                        evm_address: Some(evm_addr),
                        pubkey: pubkey_base64,
                        private_key: private_key_hex,
                        derivation_path: path,
                    });
                }
            }

            if i % 1000 == 0 {
                progress.fetch_add(1000, Ordering::Relaxed);
            }
        }
    });

    progress.fetch_add(count % 1000, Ordering::Relaxed);
    wallets
}