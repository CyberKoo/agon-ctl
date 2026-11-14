use crate::platform::NativeDisplay;
use agon_osd::{Feature, ValueDomain, VcpValue};

pub(crate) fn displays(displays: &[NativeDisplay]) {
    if displays.is_empty() {
        println!("No DDC-capable external displays found.");
        return;
    }
    for display in displays {
        let info = display.info();
        println!(
            "id={} name={} vendor={:04x} model={:04x} serial={} location={}",
            info.id(),
            info.name(),
            info.vendor_id(),
            info.model_id(),
            info.serial_number(),
            info.location()
        );
    }
}

pub(crate) fn display(display: &NativeDisplay) {
    let info = display.info();
    println!("display: {} (id={})", info.name(), info.id());
}

pub(crate) fn display_choices(displays: &[NativeDisplay]) -> String {
    displays
        .iter()
        .map(|display| {
            let info = display.info();
            format!("{} ({})", info.name(), info.id())
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn features() {
    for feature in Feature::ALL {
        let definition = feature.definition();
        print!("{:<24} {:<12} ", definition.name, definition.address);
        match definition.domain {
            ValueDomain::Number => println!("number"),
            ValueDomain::Named(values) => println!("{}", joined_names(values)),
            ValueDomain::NamedByMaximum(selector) => {
                let values = selector
                    .variants()
                    .iter()
                    .map(|variant| {
                        let condition = match variant.maximum {
                            agon_osd::NamedValueMaximum::AtMost(maximum) => {
                                format!("max<={maximum}")
                            }
                            agon_osd::NamedValueMaximum::Equal(maximum) => {
                                format!("max={maximum}")
                            }
                            agon_osd::NamedValueMaximum::Any => "other max".to_owned(),
                        };
                        format!("{condition}: {}", joined_names(variant.values))
                    })
                    .collect::<Vec<_>>()
                    .join(" | ");
                println!("{values}");
            }
        }
    }
}

fn joined_names(values: &[agon_osd::NamedValue]) -> String {
    values
        .iter()
        .map(|value| value.name)
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn value(feature: Feature, value: VcpValue) {
    let definition = feature.definition();
    if let Some(name) = definition.value_name_for_maximum(value.current, value.maximum) {
        println!(
            "{}: current={} ({}) max={}",
            definition.name, value.current, name, value.maximum
        );
    } else {
        println!(
            "{}: current={} max={}",
            definition.name, value.current, value.maximum
        );
    }
}

pub(crate) fn capability_value(feature: Feature, value: VcpValue, capabilities: &[u8]) {
    let definition = feature.definition();
    let support = feature
        .support_path()
        .map_or("not-applicable", |path| path.as_str());
    let capabilities = joined_capabilities(capabilities);
    if let Some(name) = definition.value_name_for_maximum(value.current, value.maximum) {
        println!(
            "{}: current={} ({}) max={} support={} capabilities=[{}]",
            definition.name, value.current, name, value.maximum, support, capabilities
        );
    } else {
        println!(
            "{}: current={} max={} support={} capabilities=[{}]",
            definition.name, value.current, value.maximum, support, capabilities
        );
    }
}

fn joined_capabilities(capabilities: &[u8]) -> String {
    capabilities
        .iter()
        .map(u8::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_list_uses_decimal_values_and_preserves_order() {
        assert_eq!(joined_capabilities(&[0, 2, 4, 138, 2]), "0, 2, 4, 138, 2");
        assert_eq!(joined_capabilities(&[]), "");
    }
}
