//! Character encoding for imported text files.
//!
//! A novel downloaded years ago is often GB18030/GBK rather than UTF-8, and
//! importing it as UTF-8 would either fail outright or, worse, turn into
//! replacement characters that the analysis then counts as prose. So the bytes
//! are decoded explicitly and the encoding actually used is reported back for
//! the import log.

use studio_core::error::{Result, StudioError};

/// Decoded text and the name of the encoding it was decoded as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    pub text: String,
    pub encoding: &'static str,
}

/// Decode a text file's bytes, trying UTF-8 first and GB18030 second.
pub fn decode_text_file(bytes: &[u8]) -> Result<Decoded> {
    // A byte-order mark is the file stating its own encoding; believe it.
    if let Some(body) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return Ok(Decoded {
            text: String::from_utf8_lossy(body).into_owned(),
            encoding: "UTF-8 (BOM)",
        });
    }
    for (bom, encoding, name) in [
        (&[0xFF, 0xFE][..], encoding_rs::UTF_16LE, "UTF-16LE"),
        (&[0xFE, 0xFF][..], encoding_rs::UTF_16BE, "UTF-16BE"),
    ] {
        if bytes.starts_with(bom) {
            let (text, _, had_errors) = encoding.decode(&bytes[bom.len()..]);
            if had_errors {
                return Err(StudioError::Import(format!(
                    "{name} 文件在 BOM 之后有无法解码的字节"
                )));
            }
            return Ok(Decoded {
                text: text.into_owned(),
                encoding: name,
            });
        }
    }

    if let Ok(text) = std::str::from_utf8(bytes) {
        return Ok(Decoded {
            text: text.to_string(),
            encoding: "UTF-8",
        });
    }

    // GB18030 is a superset of GBK and GB2312, so one attempt covers all three.
    let (text, _, had_errors) = encoding_rs::GB18030.decode(bytes);
    if had_errors {
        return Err(StudioError::Import(
            "无法识别文件编码（既不是 UTF-8，也不是 GB18030/GBK/GB2312）。请先把文件转成 UTF-8 再导入。"
                .to_string(),
        ));
    }
    Ok(Decoded {
        text: text.into_owned(),
        encoding: "GB18030",
    })
}

/// The format implied by a file name, `None` when the extension is not one this
/// importer reads.
pub fn format_of_file_name(name: &str) -> Option<crate::plaintext::TextFormat> {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "md" | "markdown" | "mdown" | "mkd" => Some(crate::plaintext::TextFormat::Markdown),
        "txt" => Some(crate::plaintext::TextFormat::Txt),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utf8_without_bom() {
        let d = decode_text_file("出版学".as_bytes()).unwrap();
        assert_eq!(d.text, "出版学");
        assert_eq!(d.encoding, "UTF-8");
    }

    #[test]
    fn test_utf8_with_bom_is_stripped() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend("出版".as_bytes());
        let d = decode_text_file(&bytes).unwrap();
        assert_eq!(d.text, "出版");
        assert_eq!(d.encoding, "UTF-8 (BOM)");
    }

    #[test]
    fn test_gb18030_is_detected() {
        // "出版" in GBK, which GB18030 accepts.
        let bytes = [0xB3, 0xF6, 0xB0, 0xE6];
        let d = decode_text_file(&bytes).unwrap();
        assert_eq!(d.text, "出版");
        assert_eq!(d.encoding, "GB18030");
    }

    #[test]
    fn test_unidentifiable_encoding_is_refused() {
        // 0x80-0x81 followed by an invalid trail byte: not valid UTF-8 either.
        let err = decode_text_file(&[0xFF, 0x80, 0x00, 0xFF]).unwrap_err();
        assert!(err.to_string().contains("编码"));
    }

    #[test]
    fn test_format_from_extension() {
        use crate::plaintext::TextFormat;
        assert_eq!(format_of_file_name("书.MD"), Some(TextFormat::Markdown));
        assert_eq!(
            format_of_file_name("novel.markdown"),
            Some(TextFormat::Markdown)
        );
        assert_eq!(format_of_file_name("book.txt"), Some(TextFormat::Txt));
        assert_eq!(format_of_file_name("data.json"), None);
    }
}
