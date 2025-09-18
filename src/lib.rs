pub mod wallet;
pub mod generator;
pub mod cli;

pub use wallet::Wallet;
pub use generator::{generate_wallets_batch, generate_addresses};
pub use cli::{Args, KeyType};