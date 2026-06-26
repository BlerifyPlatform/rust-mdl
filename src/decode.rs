//! Decode a hex-encoded ISO 18013-5 mdoc into an inspectable JSON tree.
//!
//! The Issuance API returns the assembled credential as a hex string of CBOR
//! (`DeviceResponse`: a map of `status` / `version` / `documents`). This module
//! turns that into a [`serde_json::Value`] so callers can read the structure
//! without a full typed mdoc model.
//!
//! The conversion is intentionally schema-agnostic — it does not assume any
//! mdoc field names. It only applies two transformations grounded in
//! RFC 8949 / CBOR itself:
//!
//! * **Tag 24** (`#6.24`, "encoded CBOR data item") wrapping a byte string is
//!   re-parsed as CBOR and inlined. ISO 18013-5 wraps each `IssuerSignedItem`
//!   and the `MobileSecurityObject` payload this way, so unwrapping it surfaces
//!   the data elements (e.g. an issuer's `verificationProof`) as readable JSON
//!   instead of an opaque byte blob.
//! * **Byte strings** that are not tag-24 CBOR are rendered as lowercase hex
//!   (COSE signatures, salts, key bytes, etc.).
//!
//! All other tags are preserved as `{"tag": <n>, "value": <inner>}` so nothing
//! is silently dropped.

use ciborium::value::Value as Cbor;
use serde_json::{Map, Value as Json};

use crate::error::BlerifyError;

/// CBOR tag 24 — "encoded CBOR data item" (RFC 8949 §3.4.5.1).
const TAG_ENCODED_CBOR: u64 = 24;

/// Decode a hex-encoded mdoc string into a [`serde_json::Value`] tree.
///
/// Hex decoding and CBOR parsing failures are reported as
/// [`BlerifyError::Decode`]. Tag-24 embedded CBOR is unwrapped recursively (see
/// the module docs); a tag-24 payload that does not itself parse as CBOR is
/// left as a hex byte string rather than failing the whole decode.
pub fn decode_mdoc(mdoc_hex: &str) -> Result<Json, BlerifyError> {
    let bytes =
        hex::decode(mdoc_hex.trim()).map_err(|e| BlerifyError::Decode(format!("hex: {e}")))?;
    let cbor: Cbor = ciborium::de::from_reader(bytes.as_slice())
        .map_err(|e| BlerifyError::Decode(format!("cbor: {e}")))?;
    Ok(cbor_to_json(cbor))
}

/// Faithfully convert a parsed CBOR value into JSON (see module docs for the
/// tag-24 and byte-string handling).
fn cbor_to_json(value: Cbor) -> Json {
    match value {
        Cbor::Null => Json::Null,
        Cbor::Bool(b) => Json::Bool(b),
        Cbor::Text(s) => Json::String(s),
        Cbor::Integer(i) => {
            let n = i128::from(i);
            // serde_json numbers cover i64/u64; widen to string beyond that.
            if let Ok(v) = i64::try_from(n) {
                Json::Number(v.into())
            } else if let Ok(v) = u64::try_from(n) {
                Json::Number(v.into())
            } else {
                Json::String(n.to_string())
            }
        }
        Cbor::Float(f) => serde_json::Number::from_f64(f)
            .map(Json::Number)
            // NaN / ±Inf are not representable in JSON — keep them visible.
            .unwrap_or_else(|| Json::String(f.to_string())),
        Cbor::Bytes(b) => Json::String(hex::encode(b)),
        Cbor::Array(items) => Json::Array(items.into_iter().map(cbor_to_json).collect()),
        Cbor::Map(entries) => {
            let mut map = Map::with_capacity(entries.len());
            for (k, v) in entries {
                map.insert(cbor_key_to_string(k), cbor_to_json(v));
            }
            Json::Object(map)
        }
        Cbor::Tag(tag, inner) => {
            if tag == TAG_ENCODED_CBOR {
                if let Cbor::Bytes(ref b) = *inner {
                    if let Ok(inner_cbor) = ciborium::de::from_reader::<Cbor, _>(b.as_slice()) {
                        return cbor_to_json(inner_cbor);
                    }
                }
            }
            // Preserve any other tag (or an unparseable tag-24) without losing data.
            let mut map = Map::with_capacity(2);
            map.insert("tag".to_string(), Json::Number(tag.into()));
            map.insert("value".to_string(), cbor_to_json(*inner));
            Json::Object(map)
        }
        // `ciborium::Value` is #[non_exhaustive]; surface any future variant
        // as a string rather than dropping it.
        other => Json::String(format!("{other:?}")),
    }
}

/// JSON object keys must be strings; CBOR map keys may be integers, booleans,
/// etc. Render them deterministically so structural keys (e.g. COSE integer
/// labels) survive the conversion.
fn cbor_key_to_string(key: Cbor) -> String {
    match key {
        Cbor::Text(s) => s,
        Cbor::Integer(i) => i128::from(i).to_string(),
        Cbor::Bool(b) => b.to_string(),
        Cbor::Bytes(b) => hex::encode(b),
        other => cbor_to_json(other).to_string(),
    }
}

impl crate::assemble::AssembleResponse {
    /// Decode this response's hex mdoc into an inspectable JSON tree.
    ///
    /// Convenience wrapper over [`decode_mdoc`]; see its docs for the
    /// tag-24 unwrapping behaviour.
    pub fn decode(&self) -> Result<Json, BlerifyError> {
        decode_mdoc(&self.mdoc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ciborium::value::Value as Cbor;

    /// Encode a CBOR value to bytes (test helper).
    fn enc(value: &Cbor) -> Vec<u8> {
        let mut buf = Vec::new();
        ciborium::ser::into_writer(value, &mut buf).expect("cbor encode");
        buf
    }

    #[test]
    fn decodes_minimal_device_response() {
        // DeviceResponse-shaped map: status / version / documents.
        let resp = Cbor::Map(vec![
            (Cbor::Text("status".into()), Cbor::Integer(0.into())),
            (Cbor::Text("version".into()), Cbor::Text("1.0".into())),
            (Cbor::Text("documents".into()), Cbor::Array(vec![])),
        ]);
        let json = decode_mdoc(&hex::encode(enc(&resp))).unwrap();
        assert_eq!(json["status"], serde_json::json!(0));
        assert_eq!(json["version"], serde_json::json!("1.0"));
        assert_eq!(json["documents"], serde_json::json!([]));
    }

    #[test]
    fn unwraps_tag24_embedded_cbor() {
        // An IssuerSignedItem-shaped map, wrapped as #6.24(bstr) like the
        // entries inside issuerSigned.nameSpaces.
        let item = Cbor::Map(vec![
            (
                Cbor::Text("elementIdentifier".into()),
                Cbor::Text("verificationProof".into()),
            ),
            (
                Cbor::Text("elementValue".into()),
                Cbor::Map(vec![
                    (
                        Cbor::Text("did".into()),
                        Cbor::Text("did:example:123".into()),
                    ),
                    (
                        Cbor::Text("domain".into()),
                        Cbor::Text("zEncodedPointer".into()),
                    ),
                ]),
            ),
        ]);
        let wrapped = Cbor::Tag(24, Box::new(Cbor::Bytes(enc(&item))));
        let outer = Cbor::Array(vec![wrapped]);

        let json = decode_mdoc(&hex::encode(enc(&outer))).unwrap();
        let decoded = &json[0];
        assert_eq!(
            decoded["elementIdentifier"],
            serde_json::json!("verificationProof")
        );
        assert_eq!(
            decoded["elementValue"]["did"],
            serde_json::json!("did:example:123")
        );
        assert_eq!(
            decoded["elementValue"]["domain"],
            serde_json::json!("zEncodedPointer")
        );
    }

    #[test]
    fn renders_plain_bytes_as_hex() {
        let json = decode_mdoc(&hex::encode(enc(&Cbor::Bytes(vec![
            0xde, 0xad, 0xbe, 0xef,
        ]))))
        .unwrap();
        assert_eq!(json, serde_json::json!("deadbeef"));
    }

    #[test]
    fn preserves_non_24_tags() {
        // Tag 1004 (full-date) wrapping a date string — kept, not dropped.
        let tagged = Cbor::Tag(1004, Box::new(Cbor::Text("1987-03-15".into())));
        let json = decode_mdoc(&hex::encode(enc(&tagged))).unwrap();
        assert_eq!(json["tag"], serde_json::json!(1004));
        assert_eq!(json["value"], serde_json::json!("1987-03-15"));
    }

    #[test]
    fn invalid_hex_is_decode_error() {
        let err = decode_mdoc("nothex").unwrap_err();
        assert!(matches!(err, BlerifyError::Decode(_)));
    }
}
