use veil_clip::detector::Detector;

// 合成文字列のみ。認証情報を発行せず、OSにもアクセスしない。
// Synthetic strings only; no provisioned credentials or OS access.
const AWS_ID: &str = "AKIA0000000000000000";
const AWS_IDS: [&str; 2] = [AWS_ID, "ASIA0000000000000000"];

#[test]
fn detects_literal_ids_in_json_values_and_url_components() {
    let detector = Detector::new().unwrap();
    for id in AWS_IDS {
        for text in [
            format!(r#"{{"access_key_id":"{id}"}}"#),
            format!(r#"{{"credentials":{{"ids":["{id}"]}}}}"#),
            format!("https://example.invalid/?access_key_id={id}&next=1"),
            format!("https://example.invalid/{id}/details"),
            format!("https://example.invalid/#{id}"),
        ] {
            let result = detector.detect(&text);
            assert!(result.aws_access_key_id, "synthetic input: {text:?}");
            assert!(!result.private_key_header);
        }
    }
}

#[test]
fn accepts_each_ascii_non_identifier_boundary_on_either_side() {
    let detector = Detector::new().unwrap();
    // 空白・引用符・スラッシュだけの許可リストではなく、ASCII全域を確認する。
    // Cover the complete ASCII domain, including controls and punctuation.
    for id in AWS_IDS {
        for byte in 0u8..=127 {
            if byte.is_ascii_alphanumeric() || byte == b'_' {
                continue;
            }
            let delimiter = char::from(byte);
            for text in [
                format!("{delimiter}{id}"),
                format!("{id}{delimiter}"),
                format!("{delimiter}{id}{delimiter}"),
            ] {
                assert!(
                    detector.detect(&text).aws_access_key_id,
                    "prefix {}, delimiter {byte:#04x}",
                    &id[..4]
                );
            }
        }
    }
}

#[test]
fn rejects_each_ascii_identifier_boundary_on_either_side() {
    let detector = Detector::new().unwrap();
    for id in AWS_IDS {
        for adjacent in ('a'..='z')
            .chain('A'..='Z')
            .chain('0'..='9')
            .chain(std::iter::once('_'))
        {
            for text in [
                format!("{adjacent}{id}"),
                format!("{id}{adjacent}"),
                format!("{adjacent}{id}{adjacent}"),
            ] {
                assert!(
                    detector.detect(&text).is_empty(),
                    "prefix {}, adjacent {adjacent:?}",
                    &id[..4]
                );
            }
        }
    }
}

#[test]
fn accepts_non_ascii_boundaries_without_unicode_identifier_validation() {
    let detector = Detector::new().unwrap();
    for id in AWS_IDS {
        for delimiter in ["日本語", "é", "９", "＿", "\u{0301}", "\u{00a0}", "🔑"] {
            for text in [
                format!("{delimiter}{id}"),
                format!("{id}{delimiter}"),
                format!("{delimiter}{id}{delimiter}"),
            ] {
                assert!(detector.detect(&text).aws_access_key_id);
            }
        }
    }
}

#[test]
fn scans_raw_text_without_decoding_ids_or_boundaries() {
    let detector = Detector::new().unwrap();
    for id in AWS_IDS {
        // 各文字を個別にエンコードしても、復号してIDを組み立てない。
        // Encoding any one character must not reconstruct a matching ID.
        for (index, byte) in id.bytes().enumerate() {
            let json_id = format!("{}\\u{byte:04X}{}", &id[..index], &id[index + 1..]);
            let url_id = format!("{}%{byte:02X}{}", &id[..index], &id[index + 1..]);
            assert!(detector
                .detect(&format!(r#"{{"access_key_id":"{json_id}"}}"#))
                .is_empty());
            assert!(detector
                .detect(&format!("https://example.invalid/?id={url_id}"))
                .is_empty());
        }
        // 境界も復号しない。直前の0やFはASCII識別子文字なので除外する。
        // Encoded delimiters end in identifier bytes before the literal ID.
        for text in [
            format!(r#"{{"id":"prefix\u0020{id}"}}"#),
            format!("https://example.invalid/?id=%20{id}"),
            format!("https://example.invalid/?id=%2F{id}"),
        ] {
            assert!(detector.detect(&text).is_empty());
        }
        // 生のIDがあれば、周囲にエスケープがあっても通常の境界規則で検知する。
        // Encoding elsewhere does not disable scanning of a literal ID.
        for text in [
            format!(r#"{{"id":"{id}\u0041"}}"#),
            format!("https://example.invalid/?id={id}%41"),
            format!("https://example.invalid/?label=%20&id={id}"),
        ] {
            assert!(detector.detect(&text).aws_access_key_id);
        }
    }
}

#[test]
fn detects_defined_aws_prefixes_and_ascii_boundaries() {
    let detector = Detector::new().unwrap();
    for text in [
        AWS_ID.to_owned(),
        "ASIA0000000000000000".to_owned(),
        format!("export AWS_ACCESS_KEY_ID=\"{AWS_ID}\""),
        format!("日本語「{AWS_ID}」コピー"),
        format!("\n{AWS_ID}\r\n"),
    ] {
        let result = detector.detect(&text);
        assert!(result.aws_access_key_id);
        assert!(!result.private_key_header);
    }
}

#[test]
fn rejects_short_long_lowercase_and_embedded_ids() {
    let detector = Detector::new().unwrap();
    for text in [
        "AKIA000000000000000".to_owned(),
        "AKIA00000000000000000".to_owned(),
        "akia0000000000000000".to_owned(),
        "AKIA000000000000000a".to_owned(),
        "AIDA0000000000000000".to_owned(),
        format!("a{AWS_ID}"),
        format!("9{AWS_ID}"),
        format!("_{AWS_ID}"),
        format!("{AWS_ID}z"),
        format!("{AWS_ID}9"),
        format!("{AWS_ID}_"),
    ] {
        assert!(detector.detect(&text).is_empty());
    }
}

#[test]
fn continues_search_after_a_candidate_with_invalid_boundaries() {
    let detector = Detector::new().unwrap();
    let text = format!("prefix{AWS_ID}suffix\n{AWS_ID}");
    assert!(detector.detect(&text).aws_access_key_id);
}

#[test]
fn detects_every_defined_private_key_header() {
    let detector = Detector::new().unwrap();
    for header in [
        "-----BEGIN PRIVATE KEY-----",
        "-----BEGIN ENCRYPTED PRIVATE KEY-----",
        "-----BEGIN RSA PRIVATE KEY-----",
        "-----BEGIN EC PRIVATE KEY-----",
        "-----BEGIN OPENSSH PRIVATE KEY-----",
    ] {
        let result = detector.detect(&format!("サンプル\n{header}\nsynthetic-body"));
        assert!(result.private_key_header);
        assert!(!result.aws_access_key_id);
    }
}

#[test]
fn excludes_public_keys_certificates_and_unlisted_formats() {
    let detector = Detector::new().unwrap();
    for text in [
        "",
        "通常の文章です。",
        "-----BEGIN PUBLIC KEY-----",
        "-----BEGIN RSA PUBLIC KEY-----",
        "-----BEGIN CERTIFICATE-----",
        "-----BEGIN DSA PRIVATE KEY-----",
        "-----BEGIN RSA KEY-----",
        "-----BEGIN OPENSSH KEY-----",
        "-----begin private key-----",
        "----BEGIN PRIVATE KEY----",
        "arbitrary-password-or-token",
    ] {
        assert!(detector.detect(text).is_empty());
    }
}

#[test]
fn reports_both_rules_without_excerpts_or_offsets() {
    let detector = Detector::new().unwrap();
    let result = detector.detect(&format!("{AWS_ID}\n-----BEGIN PRIVATE KEY-----"));
    assert!(result.aws_access_key_id);
    assert!(result.private_key_header);
    assert!(!format!("{result:?}").contains(AWS_ID));
}
