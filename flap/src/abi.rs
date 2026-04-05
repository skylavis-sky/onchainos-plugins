/// ABI encoding utilities for Flap contract calls.
///
/// Implements a minimal subset of Ethereum ABI encoding (EIP-712 / ABI v2)
/// sufficient for the structs used by the Flap Portal contract.

/// Encode a 20-byte Ethereum address as a 32-byte ABI word (left-padded with zeros).
pub fn encode_address(addr: &str) -> anyhow::Result<[u8; 32]> {
    let addr = addr.trim_start_matches("0x");
    if addr.len() != 40 {
        anyhow::bail!("Invalid address length: {}", addr);
    }
    let bytes = hex::decode(addr)?;
    let mut word = [0u8; 32];
    word[12..32].copy_from_slice(&bytes);
    Ok(word)
}

/// Encode a u256 as a 32-byte big-endian ABI word.
pub fn encode_uint256(value: u128) -> [u8; 32] {
    let mut word = [0u8; 32];
    let be = value.to_be_bytes();
    word[16..32].copy_from_slice(&be);
    word
}

/// Encode a u256 from a big u256 represented as two u128 halves (hi, lo).
pub fn encode_uint256_full(hi: u128, lo: u128) -> [u8; 32] {
    let mut word = [0u8; 32];
    word[0..16].copy_from_slice(&hi.to_be_bytes());
    word[16..32].copy_from_slice(&lo.to_be_bytes());
    word
}

/// Encode a u64 as a 32-byte ABI word.
pub fn encode_uint64(value: u64) -> [u8; 32] {
    encode_uint256(value as u128)
}

/// Encode a u32 as a 32-byte ABI word.
pub fn encode_uint32(value: u32) -> [u8; 32] {
    encode_uint256(value as u128)
}

/// Encode a u16 as a 32-byte ABI word.
pub fn encode_uint16(value: u16) -> [u8; 32] {
    encode_uint256(value as u128)
}

/// Encode a u8 as a 32-byte ABI word.
pub fn encode_uint8(value: u8) -> [u8; 32] {
    encode_uint256(value as u128)
}

/// Encode bytes32 (fixed-size 32-byte value, right-padded if shorter).
pub fn encode_bytes32(value: &[u8; 32]) -> [u8; 32] {
    *value
}

/// Encode a dynamic bytes value: length word + padded data.
pub fn encode_bytes_dynamic(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    // Length word
    result.extend_from_slice(&encode_uint256(data.len() as u128));
    // Data, padded to 32-byte boundary
    result.extend_from_slice(data);
    let pad = (32 - (data.len() % 32)) % 32;
    result.extend(std::iter::repeat(0u8).take(pad));
    result
}

/// Encode a dynamic string value: same as bytes.
pub fn encode_string_dynamic(s: &str) -> Vec<u8> {
    encode_bytes_dynamic(s.as_bytes())
}

/// Build the calldata for `swapExactInput((address,address,uint256,uint256,bytes))`.
///
/// ExactInputParams struct:
///   address inputToken
///   address outputToken
///   uint256 inputAmount
///   uint256 minOutputAmount
///   bytes   permitData
///
/// The struct is ABI-encoded as a tuple. Because it contains a dynamic type (bytes),
/// the encoding uses offset pointers per the ABI spec.
///
/// Calldata = selector(4) + abi.encode(tuple)
/// abi.encode(tuple) = offset_to_tuple_data(32) + tuple_data
/// tuple_data = head + tail
/// head = [address(32), address(32), uint256(32), uint256(32), offset_to_bytes(32)]
/// tail = bytes_encoded
pub fn encode_swap_exact_input(
    input_token: &str,
    output_token: &str,
    input_amount: u128,
    min_output_amount: u128,
    permit_data: &[u8],
) -> anyhow::Result<Vec<u8>> {
    use crate::config::SELECTOR_SWAP_EXACT_INPUT;

    // The outer abi.encode wraps the tuple in one level of indirection.
    // For a single tuple argument: calldata = selector + offset(32) + tuple_encoding
    // But for a struct argument in Solidity ABI, the entire encoding is:
    // selector + ABI-encoded-tuple (which is just the head+tail directly, no outer offset)
    // because structs are treated as tuples and inlined in the call.
    //
    // Actually: swapExactInput takes ONE argument which is a struct (tuple).
    // Per ABI spec, encoding a function with one tuple parameter T is:
    //   enc(T) = offset_to_T_data || T_data
    // where offset_to_T_data = 32 (points right after the offset word itself)
    // and T_data = head of T's fields + tail of T's dynamic fields.
    //
    // Fields: address(static), address(static), uint256(static), uint256(static), bytes(dynamic)
    // Head size = 5 * 32 = 160 bytes
    // bytes offset within T_data = 160 (points to tail start)

    let input_token_word = encode_address(input_token)?;
    let output_token_word = encode_address(output_token)?;
    let input_amount_word = encode_uint256(input_amount);
    let min_output_word = encode_uint256(min_output_amount);

    // offset of `bytes permitData` within the struct encoding = 5 * 32 = 160
    let bytes_offset_word = encode_uint256(160u128);

    let permit_encoded = encode_bytes_dynamic(permit_data);

    // Outer offset: the single tuple argument starts at byte 32 (right after the offset word)
    let outer_offset = encode_uint256(32u128);

    let mut calldata = Vec::new();
    calldata.extend_from_slice(&SELECTOR_SWAP_EXACT_INPUT);
    // outer offset to the struct
    calldata.extend_from_slice(&outer_offset);
    // struct head
    calldata.extend_from_slice(&input_token_word);
    calldata.extend_from_slice(&output_token_word);
    calldata.extend_from_slice(&input_amount_word);
    calldata.extend_from_slice(&min_output_word);
    calldata.extend_from_slice(&bytes_offset_word);
    // struct tail (bytes)
    calldata.extend_from_slice(&permit_encoded);

    Ok(calldata)
}

/// Build the calldata for `approve(address,uint256)`.
pub fn encode_approve(spender: &str, amount: u128) -> anyhow::Result<Vec<u8>> {
    use crate::config::SELECTOR_APPROVE;
    let mut calldata = Vec::new();
    calldata.extend_from_slice(&SELECTOR_APPROVE);
    calldata.extend_from_slice(&encode_address(spender)?);
    calldata.extend_from_slice(&encode_uint256(amount));
    Ok(calldata)
}

/// Build the calldata for `getTokenV8Safe(address)`.
pub fn encode_get_token_v8_safe(token: &str) -> anyhow::Result<Vec<u8>> {
    use crate::config::SELECTOR_GET_TOKEN_V8_SAFE;
    let mut calldata = Vec::new();
    calldata.extend_from_slice(&SELECTOR_GET_TOKEN_V8_SAFE);
    calldata.extend_from_slice(&encode_address(token)?);
    Ok(calldata)
}

/// Build the calldata for `quoteExactInput((address,address,uint256))`.
///
/// Single struct param with three static fields — no dynamic types.
/// quote_token = address(0) for BNB
pub fn encode_quote_exact_input(
    input_token: &str,
    output_token: &str,
    input_amount: u128,
) -> anyhow::Result<Vec<u8>> {
    use crate::config::SELECTOR_QUOTE_EXACT_INPUT;

    // struct is all static (no bytes/string) — encoding:
    // selector + outer_offset(32) + field1(32) + field2(32) + field3(32)
    let outer_offset = encode_uint256(32u128);
    let input_token_word = encode_address(input_token)?;
    let output_token_word = encode_address(output_token)?;
    let input_amount_word = encode_uint256(input_amount);

    let mut calldata = Vec::new();
    calldata.extend_from_slice(&SELECTOR_QUOTE_EXACT_INPUT);
    calldata.extend_from_slice(&outer_offset);
    calldata.extend_from_slice(&input_token_word);
    calldata.extend_from_slice(&output_token_word);
    calldata.extend_from_slice(&input_amount_word);
    Ok(calldata)
}

/// Build calldata for `newTokenV6((string,string,string,uint8,bytes32,uint8,address,uint256,
///   address,bytes,bytes32,bytes,uint8,uint8,uint16,uint16,uint64,uint64,uint16,uint16,
///   uint16,uint16,uint256,address,address,uint8))`.
///
/// The struct has 26 fields, several of which are dynamic (string x3, bytes x2).
/// We encode using ABI tuple encoding with offset pointers for dynamic fields.
#[allow(clippy::too_many_arguments)]
pub fn encode_new_token_v6(params: &NewTokenV6Params) -> anyhow::Result<Vec<u8>> {
    use crate::config::SELECTOR_NEW_TOKEN_V6;

    // Fields and their types:
    //  0: string   name           (dynamic)
    //  1: string   symbol         (dynamic)
    //  2: string   meta           (dynamic)
    //  3: uint8    dexThresh      (static)
    //  4: bytes32  salt           (static)
    //  5: uint8    migratorType   (static)
    //  6: address  quoteToken     (static)
    //  7: uint256  quoteAmt       (static)
    //  8: address  beneficiary    (static)
    //  9: bytes    permitData     (dynamic)
    // 10: bytes32  extensionID    (static)
    // 11: bytes    extensionData  (dynamic)
    // 12: uint8    dexId          (static)
    // 13: uint8    lpFeeProfile   (static)
    // 14: uint16   buyTaxRate     (static)
    // 15: uint16   sellTaxRate    (static)
    // 16: uint64   taxDuration    (static)
    // 17: uint64   antiFarmerDur  (static)
    // 18: uint16   mktBps         (static)
    // 19: uint16   deflationBps   (static)
    // 20: uint16   dividendBps    (static)
    // 21: uint16   lpBps          (static)
    // 22: uint256  minShareBal    (static)
    // 23: address  dividendToken  (static)
    // 24: address  commRecv       (static)
    // 25: uint8    tokenVersion   (static)
    //
    // Total fields: 26
    // Head size = 26 * 32 = 832 bytes
    // Dynamic fields: 0(name), 1(symbol), 2(meta), 9(permitData), 11(extensionData)
    // Static fields produce their values directly in head.
    // Dynamic fields produce offsets in head, data in tail.

    let head_size: u128 = 26 * 32; // 832

    // Encode all dynamic field data
    let name_data = encode_string_dynamic(&params.name);
    let symbol_data = encode_string_dynamic(&params.symbol);
    let meta_data = encode_string_dynamic(&params.meta);
    let permit_data = encode_bytes_dynamic(&params.permit_data);
    let extension_data = encode_bytes_dynamic(&params.extension_data);

    // Compute offsets (relative to start of struct encoding, i.e., from field 0)
    // offset to field 0 (name)  = head_size + 0
    let off_name: u128 = head_size;
    // offset to field 1 (symbol) = off_name + len(name_data)
    let off_symbol: u128 = off_name + name_data.len() as u128;
    // offset to field 2 (meta) = off_symbol + len(symbol_data)
    let off_meta: u128 = off_symbol + symbol_data.len() as u128;
    // offset to field 9 (permitData) = off_meta + len(meta_data)
    let off_permit: u128 = off_meta + meta_data.len() as u128;
    // offset to field 11 (extensionData) = off_permit + len(permit_data)
    let off_extension: u128 = off_permit + permit_data.len() as u128;

    let mut head = Vec::new();

    // field 0: name (dynamic -> offset)
    head.extend_from_slice(&encode_uint256(off_name));
    // field 1: symbol (dynamic -> offset)
    head.extend_from_slice(&encode_uint256(off_symbol));
    // field 2: meta (dynamic -> offset)
    head.extend_from_slice(&encode_uint256(off_meta));
    // field 3: dexThresh (uint8, static)
    head.extend_from_slice(&encode_uint8(params.dex_thresh));
    // field 4: salt (bytes32, static)
    head.extend_from_slice(&encode_bytes32(&params.salt));
    // field 5: migratorType (uint8, static)
    head.extend_from_slice(&encode_uint8(params.migrator_type));
    // field 6: quoteToken (address, static)
    head.extend_from_slice(&encode_address(&params.quote_token)?);
    // field 7: quoteAmt (uint256, static)
    head.extend_from_slice(&encode_uint256(params.quote_amt));
    // field 8: beneficiary (address, static)
    head.extend_from_slice(&encode_address(&params.beneficiary)?);
    // field 9: permitData (dynamic -> offset)
    head.extend_from_slice(&encode_uint256(off_permit));
    // field 10: extensionID (bytes32, static)
    head.extend_from_slice(&encode_bytes32(&params.extension_id));
    // field 11: extensionData (dynamic -> offset)
    head.extend_from_slice(&encode_uint256(off_extension));
    // field 12: dexId (uint8, static)
    head.extend_from_slice(&encode_uint8(params.dex_id));
    // field 13: lpFeeProfile (uint8, static)
    head.extend_from_slice(&encode_uint8(params.lp_fee_profile));
    // field 14: buyTaxRate (uint16, static)
    head.extend_from_slice(&encode_uint16(params.buy_tax_rate));
    // field 15: sellTaxRate (uint16, static)
    head.extend_from_slice(&encode_uint16(params.sell_tax_rate));
    // field 16: taxDuration (uint64, static)
    head.extend_from_slice(&encode_uint64(params.tax_duration));
    // field 17: antiFarmerDuration (uint64, static)
    head.extend_from_slice(&encode_uint64(params.anti_farmer_duration));
    // field 18: mktBps (uint16, static)
    head.extend_from_slice(&encode_uint16(params.mkt_bps));
    // field 19: deflationBps (uint16, static)
    head.extend_from_slice(&encode_uint16(params.deflation_bps));
    // field 20: dividendBps (uint16, static)
    head.extend_from_slice(&encode_uint16(params.dividend_bps));
    // field 21: lpBps (uint16, static)
    head.extend_from_slice(&encode_uint16(params.lp_bps));
    // field 22: minimumShareBalance (uint256, static)
    head.extend_from_slice(&encode_uint256(params.minimum_share_balance));
    // field 23: dividendToken (address, static)
    head.extend_from_slice(&encode_address(&params.dividend_token)?);
    // field 24: commissionReceiver (address, static)
    head.extend_from_slice(&encode_address(&params.commission_receiver)?);
    // field 25: tokenVersion (uint8, static)
    head.extend_from_slice(&encode_uint8(params.token_version));

    assert_eq!(head.len(), head_size as usize, "Head size mismatch");

    // Assemble tail
    let mut tail = Vec::new();
    tail.extend_from_slice(&name_data);
    tail.extend_from_slice(&symbol_data);
    tail.extend_from_slice(&meta_data);
    tail.extend_from_slice(&permit_data);
    tail.extend_from_slice(&extension_data);

    // The outer encoding: selector + outer_offset(32) + struct_data
    // outer_offset = 32 (struct starts right after the single offset word)
    let outer_offset = encode_uint256(32u128);

    let mut calldata = Vec::new();
    calldata.extend_from_slice(&SELECTOR_NEW_TOKEN_V6);
    calldata.extend_from_slice(&outer_offset);
    calldata.extend_from_slice(&head);
    calldata.extend_from_slice(&tail);

    Ok(calldata)
}

/// Parameters for `newTokenV6`.
#[derive(Debug, Clone)]
pub struct NewTokenV6Params {
    pub name: String,
    pub symbol: String,
    pub meta: String,
    pub dex_thresh: u8,
    pub salt: [u8; 32],
    pub migrator_type: u8,
    pub quote_token: String,   // address
    pub quote_amt: u128,
    pub beneficiary: String,   // address
    pub permit_data: Vec<u8>,
    pub extension_id: [u8; 32],
    pub extension_data: Vec<u8>,
    pub dex_id: u8,
    pub lp_fee_profile: u8,
    pub buy_tax_rate: u16,
    pub sell_tax_rate: u16,
    pub tax_duration: u64,
    pub anti_farmer_duration: u64,
    pub mkt_bps: u16,
    pub deflation_bps: u16,
    pub dividend_bps: u16,
    pub lp_bps: u16,
    pub minimum_share_balance: u128,
    pub dividend_token: String, // address
    pub commission_receiver: String, // address
    pub token_version: u8,
}

impl Default for NewTokenV6Params {
    fn default() -> Self {
        Self {
            name: String::new(),
            symbol: String::new(),
            meta: String::new(),
            dex_thresh: 0,
            salt: [0u8; 32],
            migrator_type: 0,
            quote_token: "0x0000000000000000000000000000000000000000".to_string(),
            quote_amt: 0,
            beneficiary: "0x0000000000000000000000000000000000000000".to_string(),
            permit_data: vec![],
            extension_id: [0u8; 32],
            extension_data: vec![],
            dex_id: 0,
            lp_fee_profile: 0,
            buy_tax_rate: 0,
            sell_tax_rate: 0,
            tax_duration: 0,
            anti_farmer_duration: 0,
            mkt_bps: 10000,
            deflation_bps: 0,
            dividend_bps: 0,
            lp_bps: 0,
            minimum_share_balance: 0,
            dividend_token: "0x0000000000000000000000000000000000000000".to_string(),
            commission_receiver: "0x0000000000000000000000000000000000000000".to_string(),
            token_version: 1,
        }
    }
}

/// Decode a uint256 from a 32-byte ABI word (big-endian, lower 16 bytes as u128).
pub fn decode_uint256_as_u128(word: &[u8]) -> u128 {
    if word.len() < 32 {
        return 0;
    }
    u128::from_be_bytes(word[16..32].try_into().unwrap_or([0u8; 16]))
}

/// Decode a uint256 as u64 (lower 8 bytes).
pub fn decode_uint256_as_u64(word: &[u8]) -> u64 {
    if word.len() < 32 {
        return 0;
    }
    u64::from_be_bytes(word[24..32].try_into().unwrap_or([0u8; 8]))
}

/// Decode a uint256 as u16 (lower 2 bytes).
pub fn decode_uint256_as_u16(word: &[u8]) -> u16 {
    if word.len() < 32 {
        return 0;
    }
    u16::from_be_bytes(word[30..32].try_into().unwrap_or([0u8; 2]))
}

/// Decode a uint256 as u8 (last byte).
pub fn decode_uint256_as_u8(word: &[u8]) -> u8 {
    if word.len() < 32 {
        return 0;
    }
    word[31]
}

/// Decode an address from a 32-byte ABI word (bytes 12..32).
pub fn decode_address(word: &[u8]) -> String {
    if word.len() < 32 {
        return "0x0000000000000000000000000000000000000000".to_string();
    }
    format!("0x{}", hex::encode(&word[12..32]))
}
