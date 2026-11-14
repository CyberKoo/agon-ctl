use crate::cli::Command;
use crate::output;
use crate::platform::{NativeDisplay, PlatformError, enumerate};
use agon_osd::{DdcError, Feature, Osd, OsdError};
use std::error::Error;
use std::fmt;

pub(crate) fn run(command: Command) -> Result<(), AppError> {
    match command {
        Command::List => {
            output::displays(&enumerate()?);
            Ok(())
        }
        Command::Features => {
            output::features();
            Ok(())
        }
        Command::Capabilities { display } => {
            let display = select_display(display.as_deref())?;
            output::display(&display);
            let mut osd = Osd::new(display);
            let mut supported = 0;
            let mut failures = 0;
            let capabilities = match osd.read_capabilities() {
                Ok(capabilities) => Some(capabilities.clone()),
                Err(OsdError::Ddc(
                    DdcError::Unsupported { .. }
                    | DdcError::UnsupportedFeature { .. }
                    | DdcError::UnsupportedCapabilities,
                )) => {
                    eprintln!(
                        "warning: capability string unavailable; capabilities lists are empty"
                    );
                    None
                }
                Err(error) => {
                    eprintln!("capabilities: error: {error}");
                    failures += 1;
                    None
                }
            };
            for feature in Feature::ALL {
                match osd.probe(*feature) {
                    Ok(Some(value)) => {
                        let values = capabilities
                            .as_ref()
                            .and_then(|capabilities| {
                                capabilities.listed_values(feature.definition().address)
                            })
                            .unwrap_or(&[]);
                        output::capability_value(*feature, value, values);
                        supported += 1;
                    }
                    Ok(None) => {}
                    Err(error) => {
                        eprintln!("{feature}: error: {error}");
                        failures += 1;
                    }
                }
            }
            if supported == 0 && failures == 0 {
                println!("No readable OSD features found.");
            }
            if failures == 0 {
                Ok(())
            } else {
                Err(AppError::IncompleteCapabilities(failures))
            }
        }
        Command::Get { display, feature } => {
            let display = select_display(display.as_deref())?;
            output::display(&display);
            let value = Osd::new(display).read(feature)?;
            output::value(feature, value);
            Ok(())
        }
        Command::Set {
            display,
            feature,
            value,
        } => {
            let display = select_display(display.as_deref())?;
            output::display(&display);
            let value = Osd::new(display).set_from_str(feature, &value)?;
            output::value(feature, value);
            Ok(())
        }
    }
}

fn select_display(selector: Option<&str>) -> Result<NativeDisplay, AppError> {
    let displays = enumerate()?;
    if displays.is_empty() {
        return Err(AppError::NoDisplays);
    }

    let Some(selector) = selector else {
        if displays.len() == 1 {
            return displays.into_iter().next().ok_or(AppError::NoDisplays);
        }
        return Err(AppError::AmbiguousDisplays(output::display_choices(
            &displays,
        )));
    };

    let mut matches = displays.into_iter().filter(|display| {
        let info = display.info();
        info.id() == selector || info.name().eq_ignore_ascii_case(selector)
    });
    let first = matches
        .next()
        .ok_or_else(|| AppError::DisplayNotFound(selector.to_owned()))?;
    if matches.next().is_some() {
        return Err(AppError::AmbiguousSelector(selector.to_owned()));
    }
    Ok(first)
}

#[derive(Debug)]
pub(crate) enum AppError {
    Platform(PlatformError),
    Osd(OsdError),
    NoDisplays,
    DisplayNotFound(String),
    AmbiguousDisplays(String),
    AmbiguousSelector(String),
    IncompleteCapabilities(usize),
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Platform(error) => error.fmt(formatter),
            Self::Osd(error) => error.fmt(formatter),
            Self::NoDisplays => formatter.write_str("no DDC-capable external displays found"),
            Self::DisplayNotFound(selector) => {
                write!(formatter, "display not found: {selector}")
            }
            Self::AmbiguousDisplays(choices) => write!(
                formatter,
                "multiple displays found; use --display with one of: {choices}"
            ),
            Self::AmbiguousSelector(selector) => write!(
                formatter,
                "display selector '{selector}' matches multiple displays; use its ID"
            ),
            Self::IncompleteCapabilities(count) => write!(
                formatter,
                "capability probing completed with {count} transport or response error(s)"
            ),
        }
    }
}

impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Platform(error) => Some(error),
            Self::Osd(error) => Some(error),
            _ => None,
        }
    }
}

impl From<PlatformError> for AppError {
    fn from(error: PlatformError) -> Self {
        Self::Platform(error)
    }
}

impl From<OsdError> for AppError {
    fn from(error: OsdError) -> Self {
        Self::Osd(error)
    }
}
