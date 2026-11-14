use crate::Address;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NamedValue {
    pub name: &'static str,
    pub value: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NamedValueMaximum {
    AtMost(u16),
    Equal(u16),
    Any,
}

impl NamedValueMaximum {
    pub(super) fn matches(self, maximum: u16) -> bool {
        match self {
            Self::AtMost(expected) => maximum <= expected,
            Self::Equal(expected) => maximum == expected,
            Self::Any => true,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NamedValueVariant {
    pub maximum: NamedValueMaximum,
    pub values: &'static [NamedValue],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NamedValueSelector {
    Hdr,
    LightFxMode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueDomain {
    Number,
    Named(&'static [NamedValue]),
    NamedByMaximum(NamedValueSelector),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FeatureDefinition {
    pub name: &'static str,
    pub address: Address,
    pub domain: ValueDomain,
}

impl FeatureDefinition {
    /// Resolves a name using the value table selected by the display-reported maximum.
    /// Numeric input deliberately accepts any `u16`, including unlisted values used by
    /// other firmware variants.
    pub fn parse_value_for_maximum(
        &self,
        input: &str,
        maximum: u16,
    ) -> Result<u16, ValueParseError> {
        if let Some(value) = parse_number(input) {
            return Ok(value);
        }
        parse_named(input, self.named_values(maximum)).ok_or_else(|| self.value_error(input))
    }

    pub fn named_values(&self, maximum: u16) -> &'static [NamedValue] {
        match self.domain {
            ValueDomain::Number => &[],
            ValueDomain::Named(values) => values,
            ValueDomain::NamedByMaximum(selector) => selector.values(maximum),
        }
    }

    pub fn value_name_for_maximum(&self, value: u16, maximum: u16) -> Option<&'static str> {
        value_name(self.named_values(maximum), value)
    }

    fn value_error(&self, input: &str) -> ValueParseError {
        ValueParseError {
            feature: self.name,
            input: input.to_owned(),
        }
    }
}

fn parse_named(input: &str, values: &'static [NamedValue]) -> Option<u16> {
    let normalized = normalize(input);
    values
        .iter()
        .find(|named| named.name == normalized)
        .map(|named| named.value)
}

fn value_name(values: &'static [NamedValue], value: u16) -> Option<&'static str> {
    values
        .iter()
        .find(|named| named.value == value)
        .map(|named| named.name)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueParseError {
    feature: &'static str,
    input: String,
}

impl fmt::Display for ValueParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid value '{}' for {}; expected a number or a named value supported by this feature",
            self.input, self.feature
        )
    }
}

impl Error for ValueParseError {}

pub(super) fn normalize(input: &str) -> String {
    input.trim().to_ascii_lowercase().replace('_', "-")
}

fn parse_number(input: &str) -> Option<u16> {
    let input = input.trim();
    if let Some(hex) = input
        .strip_prefix("0x")
        .or_else(|| input.strip_prefix("0X"))
    {
        u16::from_str_radix(hex, 16).ok()
    } else {
        input.parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use crate::{Feature, ValueDomain};

    #[test]
    fn named_domain_keeps_numeric_escape_hatch_for_other_models() {
        let definition = Feature::ColorGamut.definition();
        assert_eq!(definition.parse_value_for_maximum("dci-p3", 0), Ok(4));
        assert_eq!(
            definition.parse_value_for_maximum("0xFFFF", 0),
            Ok(u16::MAX)
        );
        assert_eq!(definition.parse_value_for_maximum("32768", 0), Ok(32768));
        assert!(
            definition
                .parse_value_for_maximum("not-a-gamut", 0)
                .is_err()
        );
        assert!(definition.parse_value_for_maximum("65536", 0).is_err());
        assert!(definition.parse_value_for_maximum("-1", 0).is_err());
    }

    #[test]
    fn display_scaling_preserves_all_27_non_contiguous_values() {
        let definition = Feature::DisplayScaling.definition();
        let values = definition.named_values(0);
        assert_eq!(values.len(), 27);
        assert_eq!(
            values.iter().map(|value| value.value).collect::<Vec<_>>(),
            [
                1, 2, 5, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 33, 34, 35,
                39, 40, 41, 42, 43, 44,
            ]
        );
        assert_eq!(
            definition.parse_value_for_maximum("ratio-full-scale-ratio", 0),
            Ok(5)
        );
        assert_eq!(definition.parse_value_for_maximum("ratio-27", 0), Ok(44));
        assert_eq!(
            definition.value_name_for_maximum(39, 0),
            Some("ratio-full-16-9")
        );
    }

    #[test]
    fn hdr_names_and_parsing_use_the_maximum_selected_variant() {
        let definition = Feature::HdrMode.definition();
        assert_eq!(definition.named_values(0).len(), 2);
        assert_eq!(definition.named_values(1).len(), 2);
        assert_eq!(definition.named_values(3).len(), 4);
        assert_eq!(definition.named_values(2).len(), 7);
        assert_eq!(definition.named_values(6).len(), 7);
        assert_eq!(definition.value_name_for_maximum(1, 1), Some("auto"));
        assert_eq!(definition.value_name_for_maximum(1, 3), Some("hdr-picture"));
        assert_eq!(definition.value_name_for_maximum(2, 6), Some("hdr-picture"));
        assert_eq!(definition.parse_value_for_maximum("auto", 1), Ok(1));
        assert_eq!(definition.parse_value_for_maximum("HDR_Picture", 3), Ok(1));
        assert_eq!(definition.parse_value_for_maximum("hdr-picture", 6), Ok(2));
        assert_eq!(
            definition.parse_value_for_maximum("0xFFFF", 3),
            Ok(u16::MAX)
        );
    }

    #[test]
    fn light_fx_mode_selects_all_variants_and_rejects_unknown_maxima_for_names() {
        let definition = Feature::LightFxMode.definition();
        for (maximum, len, first_name) in [
            (12, 13, "static"),
            (13, 14, "audio"),
            (14, 15, "audio1"),
            (15, 16, "lol"),
            (16, 16, "cs"),
        ] {
            let values = definition.named_values(maximum);
            assert_eq!(values.len(), len);
            assert_eq!(values[0].name, first_name);
            assert_eq!(values[0].value, 0);
            assert_eq!(values.last().unwrap().name, "demo");
        }
        assert_eq!(definition.parse_value_for_maximum("static", 14), Ok(2));
        assert_eq!(definition.parse_value_for_maximum("static", 15), Ok(3));
        assert!(definition.named_values(11).is_empty());
        assert!(definition.named_values(17).is_empty());
        assert!(definition.parse_value_for_maximum("static", 11).is_err());
        assert_eq!(definition.parse_value_for_maximum("3", 11), Ok(3));
    }

    #[test]
    fn observed_switches_are_named_but_low_input_lag_remains_numeric() {
        for feature in [
            Feature::DialPoint,
            Feature::AdaptiveSync,
            Feature::AutoWarning,
            Feature::Lea,
            Feature::ThermalProtection,
            Feature::AutoPixelRefresh,
            Feature::RaceAssist,
            Feature::ClearSight,
            Feature::LifeGlow,
            Feature::AutoVision,
        ] {
            let values = feature.definition().named_values(u16::MAX);
            assert_eq!(values.len(), 2, "{feature}");
            assert_eq!((values[0].name, values[0].value), ("off", 0));
            assert_eq!((values[1].name, values[1].value), ("on", 1));
        }
        assert_eq!(
            Feature::LowInputLag.definition().domain,
            ValueDomain::Number
        );
    }
}
