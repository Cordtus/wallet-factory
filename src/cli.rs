use clap::{Parser, ValueEnum};

#[derive(ValueEnum, Debug, Clone)]
pub enum KeyType {
    /// Standard secp256k1 (SHA256 + RIPEMD160)
    Secp256k1,
    /// Ethereum-compatible secp256k1 (Keccak256)
    Ethsecp256k1,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Number of wallets to generate
    #[arg(short, long)]
    pub count: usize,

    /// Mnemonic phrase (will prompt if not provided)
    #[arg(short, long)]
    pub mnemonic: Option<String>,

    /// Output file path
    #[arg(short, long, default_value = "data/wallets/wallets_info.json")]
    pub output: String,

    /// Bech32 prefix for addresses
    #[arg(short, long, default_value = "cosmos")]
    pub prefix: String,

    /// Key type to generate
    #[arg(short = 'k', long, value_enum, default_value_t = KeyType::Secp256k1)]
    pub key_type: KeyType,

    /// Number of parallel threads (0 = auto-detect)
    #[arg(short, long, default_value_t = 0)]
    pub threads: usize,
}