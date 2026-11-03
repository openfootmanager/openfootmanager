//! The `report` part of an upload: the player's words and this machine's identity, redacted and
//! sized to fit the 64 KiB the server accepts.

use serde::Serialize;

use super::hub::MAX_REPORT_JSON_BYTES;
use super::redact::Redactor;

/// What the player typed on the report screen, as the separate fields the endpoint stores.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ReportFields {
    pub what_happened: String,
    pub expected: String,
    pub steps: String,
    /// The frequency choice's key (`always`, `sometimes`, ...), not its translated label.
    pub frequency: String,
    pub game_context: String,
    pub locale: String,
}

pub struct Machine<'a> {
    pub os: &'a str,
    pub arch: &'a str,
    pub webview_version: &'a str,
}

// The endpoint's per-field limits, in characters.
const TEXT_FIELD_MAX_CHARS: usize = 10_000;
const FREQUENCY_MAX_CHARS: usize = 32;
const LOCALE_MAX_CHARS: usize = 16;
const OS_MAX_CHARS: usize = 32;
const ARCH_MAX_CHARS: usize = 16;
const WEBVIEW_MAX_CHARS: usize = 64;
const DIAGNOSTICS_MAX_BYTES: usize = 64 * 1024;

fn clipped(redactor: &Redactor, text: &str, max_chars: usize) -> String {
    redactor.apply(text).chars().take(max_chars).collect()
}

fn build(
    redactor: &Redactor,
    fields: &ReportFields,
    machine: &Machine<'_>,
    diagnostics: Option<&serde_json::Value>,
    text_chars: usize,
) -> String {
    serde_json::json!({
        "what_happened": clipped(redactor, &fields.what_happened, text_chars),
        "expected": clipped(redactor, &fields.expected, text_chars),
        "steps": clipped(redactor, &fields.steps, text_chars),
        "frequency": clipped(redactor, &fields.frequency, FREQUENCY_MAX_CHARS),
        "game_context": clipped(redactor, &fields.game_context, text_chars),
        "locale": clipped(redactor, &fields.locale, LOCALE_MAX_CHARS),
        "os": clipped(redactor, machine.os, OS_MAX_CHARS),
        "arch": clipped(redactor, machine.arch, ARCH_MAX_CHARS),
        "webview_version": clipped(redactor, machine.webview_version, WEBVIEW_MAX_CHARS),
        "diagnostics": diagnostics,
    })
    .to_string()
}

/// Every field within its own limit can still add up to more than the whole part may be (four
/// full CJK fields are ~120 KB), so the part as a whole is budgeted: diagnostics go first, then
/// the long text fields are shortened together until it fits.
pub fn report_json(
    redactor: &Redactor,
    fields: &ReportFields,
    machine: &Machine<'_>,
    diagnostics: &impl Serialize,
) -> String {
    let mut diagnostics = serde_json::to_value(diagnostics)
        .ok()
        .filter(|value| value.to_string().len() <= DIAGNOSTICS_MAX_BYTES);
    let mut text_chars = TEXT_FIELD_MAX_CHARS;
    loop {
        let json = build(redactor, fields, machine, diagnostics.as_ref(), text_chars);
        if json.len() <= MAX_REPORT_JSON_BYTES || text_chars == 0 {
            return json;
        }
        if diagnostics.take().is_none() {
            text_chars /= 2;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn machine() -> Machine<'static> {
        Machine {
            os: "linux",
            arch: "x86_64",
            webview_version: "2.40",
        }
    }

    fn fields() -> ReportFields {
        ReportFields {
            what_happened: "it died loading /home/srobot/saves/a.db".to_owned(),
            expected: "to open".to_owned(),
            steps: "load".to_owned(),
            frequency: "always".to_owned(),
            game_context: "season 2".to_owned(),
            locale: "en".to_owned(),
        }
    }

    fn parsed(
        redactor: &Redactor,
        fields: &ReportFields,
        diagnostics: &impl Serialize,
    ) -> serde_json::Value {
        serde_json::from_str(&report_json(redactor, fields, &machine(), diagnostics)).expect("json")
    }

    /// Given words that mention the player's home directory,
    /// when the `report` part is built,
    /// then they are redacted like everything else that leaves the machine.
    #[test]
    fn the_report_part_is_redacted() {
        let redactor = Redactor::new(Some("/home/srobot"), Some("srobot"), None);

        let json = parsed(
            &redactor,
            &fields(),
            &serde_json::json!({ "has_active_save": true }),
        );

        assert_eq!(json["what_happened"], "it died loading ~/saves/a.db");
        assert_eq!(json["frequency"], "always");
        assert_eq!(json["os"], "linux");
        assert_eq!(json["arch"], "x86_64");
        assert_eq!(json["webview_version"], "2.40");
        assert_eq!(json["diagnostics"]["has_active_save"], true);
    }

    /// Given fields longer than the endpoint accepts,
    /// when the `report` part is built,
    /// then each is cut to its limit in characters, not bytes.
    #[test]
    fn each_field_respects_its_limit() {
        let redactor = Redactor::new(None, None, None);
        let mut long = fields();
        long.what_happened = "é".repeat(TEXT_FIELD_MAX_CHARS + 5);
        long.frequency = "f".repeat(FREQUENCY_MAX_CHARS + 5);
        long.locale = "l".repeat(LOCALE_MAX_CHARS + 5);

        let json = parsed(&redactor, &long, &serde_json::json!({}));

        let chars = |key: &str| json[key].as_str().unwrap().chars().count();
        // "é" is two bytes; the whole part still fits, so no field is shortened further.
        assert_eq!(chars("what_happened"), TEXT_FIELD_MAX_CHARS);
        assert_eq!(chars("frequency"), FREQUENCY_MAX_CHARS);
        assert_eq!(chars("locale"), LOCALE_MAX_CHARS);
    }

    /// Given diagnostics too big for the endpoint's `diagnostics` limit,
    /// when the `report` part is built,
    /// then the object is left out, and the machine's identity is still sent.
    #[test]
    fn oversized_diagnostics_are_left_out_but_the_machine_is_kept() {
        let redactor = Redactor::new(None, None, None);
        let huge = serde_json::json!({ "log_directory": "x".repeat(DIAGNOSTICS_MAX_BYTES + 1) });

        let json = parsed(&redactor, &fields(), &huge);

        assert!(json["diagnostics"].is_null());
        assert_eq!(json["os"], "linux");
        assert_eq!(json["arch"], "x86_64");
        assert_eq!(json["webview_version"], "2.40");
    }

    /// Given four full-length CJK fields, each within its own limit but 120 KB together,
    /// when the `report` part is built,
    /// then the whole part is within the server's limit, with the diagnostics dropped first.
    #[test]
    fn the_whole_part_is_budgeted_not_just_each_field() {
        let redactor = Redactor::new(None, None, None);
        let mut cjk = fields();
        for field in [
            &mut cjk.what_happened,
            &mut cjk.expected,
            &mut cjk.steps,
            &mut cjk.game_context,
        ] {
            *field = "字".repeat(TEXT_FIELD_MAX_CHARS);
        }

        let json = report_json(&redactor, &cjk, &machine(), &serde_json::json!({ "a": 1 }));

        assert!(json.len() <= MAX_REPORT_JSON_BYTES, "{} bytes", json.len());
        let value: serde_json::Value = serde_json::from_str(&json).expect("json");
        assert!(value["diagnostics"].is_null());
        assert!(value["what_happened"].as_str().unwrap().chars().count() > 1_000);
    }

    /// Given a part that fits with its diagnostics,
    /// when it is built,
    /// then the diagnostics stay and no field is shortened.
    #[test]
    fn a_part_that_fits_loses_nothing() {
        let redactor = Redactor::new(None, None, None);

        let json = parsed(&redactor, &fields(), &serde_json::json!({ "a": 1 }));

        assert_eq!(json["diagnostics"]["a"], 1);
        assert_eq!(json["steps"], "load");
    }
}
