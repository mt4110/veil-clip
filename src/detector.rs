//! 形式による検知。鍵の有効性は確認しません。
//! Format heuristics only; credentials are not validated.

use regex::Regex;

const PRIVATE_KEY_HEADERS: [&str; 5] = [
    "-----BEGIN PRIVATE KEY-----",
    "-----BEGIN ENCRYPTED PRIVATE KEY-----",
    "-----BEGIN RSA PRIVATE KEY-----",
    "-----BEGIN EC PRIVATE KEY-----",
    "-----BEGIN OPENSSH PRIVATE KEY-----",
];

/// 本文や位置を含まない検知結果 / Detection flags without text or offsets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DetectedRules {
    pub aws_access_key_id: bool,
    pub private_key_header: bool,
}

impl DetectedRules {
    pub fn is_empty(self) -> bool {
        !self.aws_access_key_id && !self.private_key_header
    }
}

/// 起動時に一度構築し、各読取で再利用します。
/// Construct once at startup and reuse for successive reads.
pub struct Detector {
    aws_access_key_id: Regex,
}

impl Detector {
    pub fn new() -> Result<Self, regex::Error> {
        Ok(Self {
            aws_access_key_id: Regex::new(r"(?:AKIA|ASIA)[A-Z0-9]{16}")?,
        })
    }

    /// 呼出側で入力のサイズを制限します / The caller enforces input size limits.
    pub fn detect(&self, text: &str) -> DetectedRules {
        let bytes = text.as_bytes();
        let aws_access_key_id = self.aws_access_key_id.find_iter(text).any(|candidate| {
            let before = candidate
                .start()
                .checked_sub(1)
                .and_then(|index| bytes.get(index));
            let after = bytes.get(candidate.end());
            !before.is_some_and(is_identifier_byte) && !after.is_some_and(is_identifier_byte)
        });

        DetectedRules {
            aws_access_key_id,
            private_key_header: PRIVATE_KEY_HEADERS
                .iter()
                .any(|header| text.contains(header)),
        }
    }
}

fn is_identifier_byte(byte: &u8) -> bool {
    byte.is_ascii_alphanumeric() || *byte == b'_'
}
