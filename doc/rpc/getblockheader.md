# `getblockheader`

If verbose is false (default), returns a string that is serialized, hex-encoded data for blockheader `hash`.

If verbose is true, returns an Object with information about blockheader `hash`.

## Usage

### Synopsis

```bash
floresta-cli getblockheader <hash> [true|false]
```

### Examples

```bash
floresta-cli getblockheader 00000000ba63ae2eeb3d2371708291b90507c3317ef957f6fcba3811cc4fe0cc
floresta-cli getblockheader 00000000ba63ae2eeb3d2371708291b90507c3317ef957f6fcba3811cc4fe0cc true
floresta-cli getblockheader 00000000ba63ae2eeb3d2371708291b90507c3317ef957f6fcba3811cc4fe0cc false
```

## Arguments

`hex` - (string, required) The block hash

`verbose` - (boolean, optional) true for a json object, false for the hex-encoded data

## Returns

### Ok Response

- for `verbose = true`:


```json5
{                                 (json object)
  "hash" : "hex",                 (string) the block hash (same as provided)
  "confirmations" : n,            (numeric) The number of confirmations, or -1 if the block is not on the main chain
  "height" : n,                   (numeric) The block height or index
  "version" : n,                  (numeric) The block version
  "versionHex" : "hex",           (string) The block version formatted in hexadecimal
  "merkleroot" : "hex",           (string) The merkle root
  "time" : xxx,                   (numeric) The block time expressed in UNIX epoch time
  "mediantime" : xxx,             (numeric) The median block time expressed in UNIX epoch time
  "nonce" : n,                    (numeric) The nonce
  "bits" : "hex",                 (string) The bits
  "difficulty" : n,               (numeric) The difficulty
  "chainwork" : "hex",            (string) Expected number of hashes required to produce the current chain
  "nTx" : n,                      (numeric) The number of transactions in the block
  "previousblockhash" : "hex",    (string) The hash of the previous block
  "nextblockhash" : "hex"         (string) The hash of the next block
}
```

- for `verbose = false`:

```bash
<version><previousblockhash><merkleroot><time><bits><nonce>
```

### Error Enum `CommandError`

Any of the error types on `rpc_types::Error`.

## Notes

- Will print the hexadecimal format unless the param `verbose` is specified to `true`.
