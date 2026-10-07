use arboard::{Clipboard, Error};
use veil_clip::engine::{ReadErrorKind, ReadOutcome};

use crate::monitor::TextReader;

pub struct MacClipboard {
    clipboard: Clipboard,
}

impl MacClipboard {
    pub fn new() -> Result<Self, ()> {
        Clipboard::new()
            .map(|clipboard| Self { clipboard })
            .map_err(|_| ())
    }
}

impl TextReader for MacClipboard {
    fn read_text(&mut self) -> ReadOutcome {
        match self.clipboard.get_text() {
            Ok(text) => ReadOutcome::Text(text),
            Err(error) => classify_error(error),
        }
    }
}

// Unknown.descriptionを表示・保存しない。空と非テキストはAPIでは区別できない。
// Never print/store Unknown.description; this API cannot distinguish empty/non-text.
fn classify_error(error: Error) -> ReadOutcome {
    match error {
        Error::ContentNotAvailable => ReadOutcome::NonText,
        Error::ClipboardOccupied => ReadOutcome::Failed(ReadErrorKind::Occupied),
        Error::ClipboardNotSupported => ReadOutcome::Failed(ReadErrorKind::Unavailable),
        _ => ReadOutcome::Failed(ReadErrorKind::Other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_errors_without_initializing_or_reading_the_clipboard() {
        assert!(matches!(
            classify_error(Error::ContentNotAvailable),
            ReadOutcome::NonText
        ));
        assert!(matches!(
            classify_error(Error::ClipboardOccupied),
            ReadOutcome::Failed(ReadErrorKind::Occupied)
        ));
        assert!(matches!(
            classify_error(Error::ClipboardNotSupported),
            ReadOutcome::Failed(ReadErrorKind::Unavailable)
        ));
        assert!(matches!(
            classify_error(Error::ConversionFailure),
            ReadOutcome::Failed(ReadErrorKind::Other)
        ));
        let result = classify_error(Error::Unknown {
            description: "sensitive-dummy-error".into(),
        });
        assert!(!format!("{result:?}").contains("sensitive-dummy-error"));
    }
}
