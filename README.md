# Cosmos Wallet Generator

Bulk wallet generator for Cosmos SDK chains supporting both standard secp256k1 and ethsecp256k1 key types.

## Key Types

- **secp256k1** (default): Standard key derivation using SHA256 + RIPEMD160
- **ethsecp256k1**: Ethereum-compatible key derivation using Keccak256

## Installation

```bash
clone this repo

cd wallet-factory && cargo build --release
```

## Usage

```bash
# Generate standard wallets (default)
./target/release/wallet-generator --count 1000 --output wallets.json

# Generate ethsecp256k1 wallets (includes EVM addresses)
./target/release/wallet-generator --count 1000 --key-type ethsecp256k1 --output wallets.json

# With specific mnemonic
./target/release/wallet-generator --count 1000 --mnemonic "your twelve word mnemonic phrase" --output wallets.json

# Custom bech32 prefix
./target/release/wallet-generator --count 1000 --prefix osmo --output wallets.json
```

## Options

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--count` | `-c` | Number of wallets to generate | Required |
| `--key-type` | `-k` | Key type: `secp256k1` or `ethsecp256k1` | `secp256k1` |
| `--mnemonic` | `-m` | BIP39 mnemonic phrase | Prompts if not provided |
| `--output` | `-o` | Output file path | `data/wallets/wallets_info.json` |
| `--prefix` | `-p` | Bech32 address prefix | `cosmos` |
| `--threads` | `-t` | Thread count (0 = auto) | Auto-detect |

## Output Format

### Standard secp256k1
```json
{
  "address": "cosmos1...",
  "pubkey": "base64_encoded_compressed_pubkey",
  "privateKey": "hex_encoded_private_key",
  "derivationPath": "m/44'/118'/0'/0/0"
}
```

### ethsecp256k1
```json
{
  "address": "cosmos1...",
  "evmAddress": "0x...",
  "pubkey": "base64_encoded_compressed_pubkey",
  "privateKey": "hex_encoded_private_key",
  "derivationPath": "m/44'/118'/0'/0/0"
}
```

## Workflows

### Key Derivation
- BIP39 mnemonic → seed (PBKDF2, 2048 iterations)
- BIP44 HD path: `m/44'/118'/0'/0/{index}`
- secp256k1 curve

### Address Generation

**Standard secp256k1:**
- Compressed public key (33 bytes) → SHA256 → RIPEMD160 → bech32

**ethsecp256k1:**
- Uncompressed public key (65 bytes) → remove 0x04 prefix → Keccak256 → last 20 bytes → bech32 + hex

## Security

Output files contain unencrypted private keys. For testing and educational purposes only. Do not use working mainnet mnemonics!!

## License

MIT
