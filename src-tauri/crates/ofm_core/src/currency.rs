use serde::Serialize;

pub const DEFAULT_CURRENCY_CODE: &str = "EUR";

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct CurrencyDefinition {
    pub code: &'static str,
    pub symbol: &'static str,
    pub exchange_rate: f64,
}

const SUPPORTED_CURRENCIES: [CurrencyDefinition; 3] = [
    CurrencyDefinition {
        code: "EUR",
        symbol: "€",
        exchange_rate: 1.0,
    },
    CurrencyDefinition {
        code: "GBP",
        symbol: "£",
        exchange_rate: 0.86,
    },
    CurrencyDefinition {
        code: "USD",
        symbol: "$",
        exchange_rate: 1.08,
    },
];

pub fn supported_currencies() -> Vec<CurrencyDefinition> {
    SUPPORTED_CURRENCIES.to_vec()
}

pub fn normalize_currency_code(code: &str) -> Option<&'static str> {
    match code.trim().to_ascii_uppercase().as_str() {
        "EUR" => Some("EUR"),
        "GBP" => Some("GBP"),
        "USD" => Some("USD"),
        _ => None,
    }
}

pub fn currency_definition(code: &str) -> Option<CurrencyDefinition> {
    let normalized = normalize_currency_code(code)?;
    SUPPORTED_CURRENCIES
        .iter()
        .copied()
        .find(|currency| currency.code == normalized)
}

pub fn convert_amount(amount: i64, code: &str) -> Option<i64> {
    let rate = currency_definition(code)?.exchange_rate;
    Some((amount as f64 * rate).round() as i64)
}

fn convert_unsigned_amount(amount: u64, code: &str) -> Option<u64> {
    let rate = currency_definition(code)?.exchange_rate;
    Some(
        ((amount as f64) * rate)
            .round()
            .clamp(0.0, u64::MAX as f64) as u64,
    )
}

pub fn format_compact_number(amount: u64, code: &str) -> Option<String> {
    let converted = convert_unsigned_amount(amount, code)?;

    if converted >= 1_000_000 {
        Some(format!("{:.1}M", converted as f64 / 1_000_000.0))
    } else if converted >= 1_000 {
        Some(format!("{}K", converted / 1_000))
    } else {
        Some(converted.to_string())
    }
}

pub fn format_compact_money(amount: u64, code: &str) -> Option<String> {
    let currency = currency_definition(code)?;
    Some(format!(
        "{}{}",
        currency.symbol,
        format_compact_number(amount, code)?
    ))
}

pub fn default_currency_symbol() -> &'static str {
    currency_definition(DEFAULT_CURRENCY_CODE)
        .map(|currency| currency.symbol)
        .unwrap_or("€")
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_CURRENCY_CODE, convert_amount, currency_definition, default_currency_symbol,
        format_compact_money, format_compact_number, normalize_currency_code,
        supported_currencies,
    };

    #[test]
    fn returns_supported_currency_catalog() {
        let catalog = supported_currencies();

        assert_eq!(catalog.len(), 3);
        assert_eq!(catalog[0].code, DEFAULT_CURRENCY_CODE);
        assert_eq!(catalog[1].code, "GBP");
        assert_eq!(catalog[2].code, "USD");
    }

    #[test]
    fn normalizes_supported_currency_codes() {
        assert_eq!(normalize_currency_code(" eur "), Some("EUR"));
        assert_eq!(normalize_currency_code("gBp"), Some("GBP"));
        assert_eq!(normalize_currency_code("USD"), Some("USD"));
    }

    #[test]
    fn rejects_unsupported_currency_codes() {
        assert_eq!(normalize_currency_code("cad"), None);
        assert!(currency_definition("cad").is_none());
        assert!(convert_amount(1_000, "cad").is_none());
        assert!(format_compact_money(1_000, "cad").is_none());
    }

    #[test]
    fn converts_amounts_using_supported_exchange_rates() {
        assert_eq!(convert_amount(125_000, "EUR"), Some(125_000));
        assert_eq!(convert_amount(125_000, "GBP"), Some(107_500));
        assert_eq!(convert_amount(125_000, "USD"), Some(135_000));
    }

    #[test]
    fn formats_compact_amounts_after_conversion() {
        assert_eq!(format_compact_number(999, "GBP"), Some("859".to_string()));
        assert_eq!(format_compact_money(125_000, "GBP"), Some("£107K".to_string()));
        assert_eq!(
            format_compact_money(5_000_000, "GBP"),
            Some("£4.3M".to_string())
        );
        assert_eq!(
            format_compact_money(1_250_000, "USD"),
            Some("$1.4M".to_string())
        );
    }

    #[test]
    fn exposes_the_default_currency_symbol() {
        assert_eq!(default_currency_symbol(), "€");
    }
}
