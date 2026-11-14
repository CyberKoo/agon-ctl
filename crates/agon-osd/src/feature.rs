mod support;
mod value;
mod values;

use crate::Address;
use std::fmt;
use value::normalize;
use values::*;

pub use support::SupportPath;
pub use value::{
    FeatureDefinition, NamedValue, NamedValueMaximum, NamedValueSelector, NamedValueVariant,
    ValueDomain, ValueParseError,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Feature {
    Brightness,
    Contrast,
    Sharpness,
    ColorPreset,
    InputSource,
    DisplayScaling,
    Volume,
    Mute,
    GameMode,
    Gamma,
    Overdrive,
    HdrMode,
    DcbMode,
    EcoMode,
    LowBlueMode,
    LowInputLag,
    ShadowControl,
    LightFx,
    LightFxMode,
    DialPoint,
    FrameCounter,
    SaturationRed,
    SaturationYellow,
    SaturationGreen,
    SaturationCyan,
    SaturationBlue,
    SaturationMagenta,
    HueRed,
    HueYellow,
    HueGreen,
    HueCyan,
    HueBlue,
    HueMagenta,
    ColorGamut,
    LocalDimming,
    ShadowBoost,
    SharpShooterSize,
    PixelOrbiting,
    ScreenSaver,
    MultiLogoProtection,
    BoundaryDimmer,
    TaskbarDimmer,
    AdaptiveSync,
    AutoWarning,
    Lea,
    ThermalProtection,
    AutoPixelRefresh,
    RaceAssist,
    Spotlight,
    NightSight,
    ClearSight,
    LifeGlow,
    AutoVision,
    AudioOutput,
    DaisyChain,
}

impl Feature {
    pub const ALL: &'static [Self] = &[
        Self::Brightness,
        Self::Contrast,
        Self::Sharpness,
        Self::ColorPreset,
        Self::InputSource,
        Self::DisplayScaling,
        Self::Volume,
        Self::Mute,
        Self::GameMode,
        Self::Gamma,
        Self::Overdrive,
        Self::HdrMode,
        Self::DcbMode,
        Self::EcoMode,
        Self::LowBlueMode,
        Self::LowInputLag,
        Self::ShadowControl,
        Self::LightFx,
        Self::LightFxMode,
        Self::DialPoint,
        Self::FrameCounter,
        Self::SaturationRed,
        Self::SaturationYellow,
        Self::SaturationGreen,
        Self::SaturationCyan,
        Self::SaturationBlue,
        Self::SaturationMagenta,
        Self::HueRed,
        Self::HueYellow,
        Self::HueGreen,
        Self::HueCyan,
        Self::HueBlue,
        Self::HueMagenta,
        Self::ColorGamut,
        Self::LocalDimming,
        Self::ShadowBoost,
        Self::SharpShooterSize,
        Self::PixelOrbiting,
        Self::ScreenSaver,
        Self::MultiLogoProtection,
        Self::BoundaryDimmer,
        Self::TaskbarDimmer,
        Self::AdaptiveSync,
        Self::AutoWarning,
        Self::Lea,
        Self::ThermalProtection,
        Self::AutoPixelRefresh,
        Self::RaceAssist,
        Self::Spotlight,
        Self::NightSight,
        Self::ClearSight,
        Self::LifeGlow,
        Self::AutoVision,
        Self::AudioOutput,
        Self::DaisyChain,
    ];

    pub fn from_name(name: &str) -> Option<Self> {
        let name = normalize(name);
        Self::ALL
            .iter()
            .copied()
            .find(|feature| feature.definition().name == name)
    }

    pub const fn definition(self) -> FeatureDefinition {
        match self {
            Self::Brightness => numeric("brightness", Address::Standard(0x10)),
            Self::Contrast => numeric("contrast", Address::Standard(0x12)),
            Self::Sharpness => numeric("sharpness", Address::Standard(0x87)),
            Self::ColorPreset => named("color-preset", Address::Standard(0x14), COLOR_PRESET),
            Self::InputSource => named("input-source", Address::Standard(0x60), INPUT_SOURCE),
            Self::DisplayScaling => {
                named("display-scaling", Address::Standard(0x86), DISPLAY_SCALING)
            }
            Self::Volume => numeric("volume", Address::Standard(0x62)),
            Self::Mute => named("mute", Address::Standard(0x8D), MUTE),
            Self::GameMode => named("game-mode", Address::Standard(0xDC), GAME_MODE),
            Self::Gamma => named("gamma", Address::f8e1(0x12), GAMMA),
            Self::Overdrive => named("overdrive", Address::f8e1(0x51), OVERDRIVE),
            Self::HdrMode => {
                named_by_maximum("hdr-mode", Address::f8e1(0x52), NamedValueSelector::Hdr)
            }
            Self::DcbMode => named("dcb-mode", Address::f8e1(0x08), DCB_MODE),
            Self::EcoMode => named("eco-mode", Address::f8e1(0x11), ECO_MODE),
            Self::LowBlueMode => named("low-blue-mode", Address::f8e1(0x59), LOW_BLUE_MODE),
            Self::LowInputLag => numeric("low-input-lag", Address::f8e1(0x55)),
            Self::ShadowControl => numeric("shadow-control", Address::f8e1(0x58)),
            Self::LightFx => named("light-fx", Address::f8e1(0x60), LIGHT_FX),
            Self::LightFxMode => named_by_maximum(
                "light-fx-mode",
                Address::f8e1(0x61),
                NamedValueSelector::LightFxMode,
            ),
            Self::DialPoint => named("dial-point", Address::f8e1(0x5A), SWITCH_FLAG),
            Self::FrameCounter => named("frame-counter", Address::f8e1(0x5B), FRAME_COUNTER),
            Self::SaturationRed => numeric("saturation-red", Address::f8e1(0x71)),
            Self::SaturationYellow => numeric("saturation-yellow", Address::f8e1(0x72)),
            Self::SaturationGreen => numeric("saturation-green", Address::f8e1(0x73)),
            Self::SaturationCyan => numeric("saturation-cyan", Address::f8e1(0x74)),
            Self::SaturationBlue => numeric("saturation-blue", Address::f8e1(0x75)),
            Self::SaturationMagenta => numeric("saturation-magenta", Address::f8e1(0x76)),
            Self::HueRed => numeric("hue-red", Address::f8e1(0x78)),
            Self::HueYellow => numeric("hue-yellow", Address::f8e1(0x79)),
            Self::HueGreen => numeric("hue-green", Address::f8e1(0x7A)),
            Self::HueCyan => numeric("hue-cyan", Address::f8e1(0x7B)),
            Self::HueBlue => numeric("hue-blue", Address::f8e1(0x7C)),
            Self::HueMagenta => numeric("hue-magenta", Address::f8e1(0x7D)),
            Self::ColorGamut => named("color-gamut", Address::e2a0(0x20), COLOR_GAMUT),
            Self::LocalDimming => named("local-dimming", Address::e2a0(0x18), LOCAL_DIMMING),
            Self::ShadowBoost => named("shadow-boost", Address::e2a0(0x45), SHADOW_BOOST),
            Self::SharpShooterSize => named(
                "sharp-shooter-size",
                Address::e2a0(0x06),
                SHARP_SHOOTER_SIZE,
            ),
            Self::PixelOrbiting => named("pixel-orbiting", Address::e2a0(0x34), PIXEL_ORBITING),
            Self::ScreenSaver => named("screen-saver", Address::e2a0(0x35), SCREEN_SAVER),
            Self::MultiLogoProtection => named(
                "multi-logo-protection",
                Address::e2a0(0x4C),
                MULTI_LOGO_PROTECTION,
            ),
            Self::BoundaryDimmer => named("boundary-dimmer", Address::e2a0(0x4D), DIMMER),
            Self::TaskbarDimmer => named("taskbar-dimmer", Address::e2a0(0x4E), DIMMER),
            Self::AdaptiveSync => named("adaptive-sync", Address::e2a0(0x40), SWITCH_FLAG),
            Self::AutoWarning => named("auto-warning", Address::e2a0(0x43), SWITCH_FLAG),
            Self::Lea => named("lea", Address::e2a0(0x46), SWITCH_FLAG),
            Self::ThermalProtection => {
                named("thermal-protection", Address::e2a0(0x4F), SWITCH_FLAG)
            }
            Self::AutoPixelRefresh => named("auto-pixel-refresh", Address::e2a0(0x61), SWITCH_FLAG),
            Self::RaceAssist => named("race-assist", Address::e2a0(0x67), SWITCH_FLAG),
            Self::Spotlight => named("spotlight", Address::e2a0(0x68), DIMMER),
            Self::NightSight => named("night-sight", Address::e2a0(0x69), DIMMER),
            Self::ClearSight => named("clear-sight", Address::e2a0(0x6A), SWITCH_FLAG),
            Self::LifeGlow => named("life-glow", Address::e2a0(0x6B), SWITCH_FLAG),
            Self::AutoVision => named("auto-vision", Address::e2a0(0x6C), SWITCH_FLAG),
            Self::AudioOutput => named("audio-output", Address::e2a0(0x7F), AUDIO_OUTPUT),
            Self::DaisyChain => named("daisy-chain", Address::e2a0(0x21), DAISY_CHAIN),
        }
    }
}

impl fmt::Display for Feature {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.definition().name)
    }
}

const fn numeric(name: &'static str, address: Address) -> FeatureDefinition {
    FeatureDefinition {
        name,
        address,
        domain: ValueDomain::Number,
    }
}

const fn named(
    name: &'static str,
    address: Address,
    values: &'static [NamedValue],
) -> FeatureDefinition {
    FeatureDefinition {
        name,
        address,
        domain: ValueDomain::Named(values),
    }
}

const fn named_by_maximum(
    name: &'static str,
    address: Address,
    selector: NamedValueSelector,
) -> FeatureDefinition {
    FeatureDefinition {
        name,
        address,
        domain: ValueDomain::NamedByMaximum(selector),
    }
}
