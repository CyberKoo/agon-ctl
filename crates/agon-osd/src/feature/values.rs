use super::NamedValue;

pub(super) const MUTE: &[NamedValue] = &[
    NamedValue {
        name: "on",
        value: 1,
    },
    NamedValue {
        name: "off",
        value: 2,
    },
];

pub(super) const COLOR_PRESET: &[NamedValue] = &[
    NamedValue {
        name: "none",
        value: 0,
    },
    NamedValue {
        name: "srgb",
        value: 1,
    },
    NamedValue {
        name: "warm",
        value: 5,
    },
    NamedValue {
        name: "normal",
        value: 6,
    },
    NamedValue {
        name: "cool",
        value: 8,
    },
    NamedValue {
        name: "user-rgb",
        value: 11,
    },
    NamedValue {
        name: "adobe-rgb",
        value: 14,
    },
];

pub(super) const INPUT_SOURCE: &[NamedValue] = &[
    NamedValue {
        name: "vga",
        value: 0x01,
    },
    NamedValue {
        name: "dvi1",
        value: 0x03,
    },
    NamedValue {
        name: "dp1",
        value: 0x0F,
    },
    NamedValue {
        name: "dp2",
        value: 0x10,
    },
    NamedValue {
        name: "hdmi1",
        value: 0x11,
    },
    NamedValue {
        name: "hdmi2",
        value: 0x12,
    },
    NamedValue {
        name: "hdmi3",
        value: 0x13,
    },
    NamedValue {
        name: "usb-c1",
        value: 0x15,
    },
    NamedValue {
        name: "usb-c2",
        value: 0x16,
    },
    NamedValue {
        name: "home",
        value: 0x19,
    },
];

pub(super) const GAME_MODE: &[NamedValue] = &[
    NamedValue {
        name: "off",
        value: 0,
    },
    NamedValue {
        name: "fps",
        value: 11,
    },
    NamedValue {
        name: "rts",
        value: 12,
    },
    NamedValue {
        name: "racing",
        value: 13,
    },
    NamedValue {
        name: "gamer1",
        value: 14,
    },
    NamedValue {
        name: "gamer2",
        value: 15,
    },
    NamedValue {
        name: "gamer3",
        value: 16,
    },
];

pub(super) const GAMMA: &[NamedValue] = &[
    NamedValue {
        name: "gamma1",
        value: 0,
    },
    NamedValue {
        name: "gamma2",
        value: 1,
    },
    NamedValue {
        name: "gamma3",
        value: 2,
    },
    NamedValue {
        name: "gamma4",
        value: 3,
    },
    NamedValue {
        name: "gamma5",
        value: 4,
    },
    NamedValue {
        name: "srgb",
        value: 5,
    },
];

pub(super) const OVERDRIVE: &[NamedValue] = &[
    NamedValue {
        name: "off",
        value: 0,
    },
    NamedValue {
        name: "weak",
        value: 1,
    },
    NamedValue {
        name: "medium",
        value: 2,
    },
    NamedValue {
        name: "strong",
        value: 3,
    },
    NamedValue {
        name: "boost",
        value: 4,
    },
];

pub(super) const DCB_MODE: &[NamedValue] = &[
    NamedValue {
        name: "off",
        value: 0,
    },
    NamedValue {
        name: "full-enhance",
        value: 33,
    },
    NamedValue {
        name: "nature-skin",
        value: 34,
    },
    NamedValue {
        name: "sky-blue",
        value: 35,
    },
    NamedValue {
        name: "green-field",
        value: 36,
    },
    NamedValue {
        name: "auto-detect",
        value: 37,
    },
    NamedValue {
        name: "demo",
        value: 38,
    },
];

pub(super) const ECO_MODE: &[NamedValue] = &[
    NamedValue {
        name: "standard",
        value: 0,
    },
    NamedValue {
        name: "text",
        value: 1,
    },
    NamedValue {
        name: "internet",
        value: 2,
    },
    NamedValue {
        name: "game",
        value: 3,
    },
    NamedValue {
        name: "movie",
        value: 4,
    },
    NamedValue {
        name: "sport",
        value: 5,
    },
    NamedValue {
        name: "reading",
        value: 6,
    },
    NamedValue {
        name: "uniformity",
        value: 7,
    },
];

pub(super) const LOW_BLUE_MODE: &[NamedValue] = &[
    NamedValue {
        name: "off",
        value: 0,
    },
    NamedValue {
        name: "multimedia",
        value: 1,
    },
    NamedValue {
        name: "internet",
        value: 2,
    },
    NamedValue {
        name: "office",
        value: 3,
    },
    NamedValue {
        name: "reading",
        value: 4,
    },
];

pub(super) const LIGHT_FX: &[NamedValue] = &[
    NamedValue {
        name: "off",
        value: 0,
    },
    NamedValue {
        name: "low",
        value: 1,
    },
    NamedValue {
        name: "medium",
        value: 2,
    },
    NamedValue {
        name: "strong",
        value: 3,
    },
];

pub(super) const FRAME_COUNTER: &[NamedValue] = &[
    NamedValue {
        name: "off",
        value: 0,
    },
    NamedValue {
        name: "top-right",
        value: 1,
    },
    NamedValue {
        name: "bottom-right",
        value: 2,
    },
    NamedValue {
        name: "bottom-left",
        value: 3,
    },
    NamedValue {
        name: "top-left",
        value: 4,
    },
];

pub(super) const COLOR_GAMUT: &[NamedValue] = &[
    NamedValue {
        name: "panel-native",
        value: 0,
    },
    NamedValue {
        name: "ntsc",
        value: 1,
    },
    NamedValue {
        name: "srgb",
        value: 2,
    },
    NamedValue {
        name: "adobe-rgb",
        value: 3,
    },
    NamedValue {
        name: "dci-p3",
        value: 4,
    },
    NamedValue {
        name: "rec-2020",
        value: 5,
    },
    NamedValue {
        name: "rec-709",
        value: 6,
    },
    NamedValue {
        name: "dmode",
        value: 7,
    },
    NamedValue {
        name: "rec-2020-hdr",
        value: 8,
    },
    NamedValue {
        name: "dci-p3-hdr",
        value: 9,
    },
    NamedValue {
        name: "off",
        value: 10,
    },
    NamedValue {
        name: "on",
        value: 11,
    },
    NamedValue {
        name: "adobe-rgb-d50",
        value: 12,
    },
    NamedValue {
        name: "dci-p3-d50",
        value: 13,
    },
    NamedValue {
        name: "display-p3",
        value: 14,
    },
    NamedValue {
        name: "cmr-srgb",
        value: 128,
    },
    NamedValue {
        name: "cmr-display-p3",
        value: 129,
    },
    NamedValue {
        name: "cmr-display-dci-p3",
        value: 130,
    },
    NamedValue {
        name: "cmr-display-dci-p3-d50",
        value: 131,
    },
    NamedValue {
        name: "cmr-adobe-rgb",
        value: 132,
    },
    NamedValue {
        name: "cmr-adobe-rgb-d50",
        value: 133,
    },
    NamedValue {
        name: "cmr-rec-2020",
        value: 134,
    },
    NamedValue {
        name: "cmr-rec-709",
        value: 135,
    },
    NamedValue {
        name: "cmr-rec-2020-hdr",
        value: 136,
    },
    NamedValue {
        name: "cmr-dci-p3-hdr",
        value: 137,
    },
    NamedValue {
        name: "cmr-custom",
        value: 138,
    },
];

pub(super) const LOCAL_DIMMING: &[NamedValue] = &[
    NamedValue {
        name: "off",
        value: 0,
    },
    NamedValue {
        name: "on",
        value: 1,
    },
    NamedValue {
        name: "weak",
        value: 2,
    },
    NamedValue {
        name: "medium",
        value: 3,
    },
    NamedValue {
        name: "strong",
        value: 4,
    },
];

pub(super) const SHADOW_BOOST: &[NamedValue] = &[
    NamedValue {
        name: "off",
        value: 0,
    },
    NamedValue {
        name: "level1",
        value: 1,
    },
    NamedValue {
        name: "level2",
        value: 2,
    },
    NamedValue {
        name: "level3",
        value: 3,
    },
    NamedValue {
        name: "dynamic",
        value: 4,
    },
];

pub(super) const SHARP_SHOOTER_SIZE: &[NamedValue] = &[
    NamedValue {
        name: "1x",
        value: 0,
    },
    NamedValue {
        name: "1.5x",
        value: 1,
    },
    NamedValue {
        name: "2x",
        value: 2,
    },
    NamedValue {
        name: "off",
        value: 3,
    },
];

pub(super) const PIXEL_ORBITING: &[NamedValue] = &[
    NamedValue {
        name: "off",
        value: 0,
    },
    NamedValue {
        name: "weak",
        value: 1,
    },
    NamedValue {
        name: "medium",
        value: 2,
    },
    NamedValue {
        name: "strong",
        value: 3,
    },
    NamedValue {
        name: "strongest",
        value: 4,
    },
];

pub(super) const SCREEN_SAVER: &[NamedValue] = &[
    NamedValue {
        name: "off",
        value: 0,
    },
    NamedValue {
        name: "on",
        value: 1,
    },
    NamedValue {
        name: "slow",
        value: 2,
    },
    NamedValue {
        name: "fast",
        value: 3,
    },
];

pub(super) const MULTI_LOGO_PROTECTION: &[NamedValue] = &[
    NamedValue {
        name: "off",
        value: 0,
    },
    NamedValue {
        name: "level1",
        value: 1,
    },
    NamedValue {
        name: "level2",
        value: 2,
    },
    NamedValue {
        name: "level3",
        value: 3,
    },
    NamedValue {
        name: "level4",
        value: 4,
    },
];

pub(super) const DIMMER: &[NamedValue] = &[
    NamedValue {
        name: "off",
        value: 0,
    },
    NamedValue {
        name: "low",
        value: 1,
    },
    NamedValue {
        name: "medium",
        value: 2,
    },
    NamedValue {
        name: "strong",
        value: 3,
    },
];

pub(super) const AUDIO_OUTPUT: &[NamedValue] = &[
    NamedValue {
        name: "speaker",
        value: 0,
    },
    NamedValue {
        name: "headphone",
        value: 1,
    },
];

pub(super) const DAISY_CHAIN: &[NamedValue] = &[
    NamedValue {
        name: "off",
        value: 0,
    },
    NamedValue {
        name: "clone",
        value: 1,
    },
    NamedValue {
        name: "extend",
        value: 2,
    },
];

use super::value::{NamedValueMaximum, NamedValueSelector, NamedValueVariant};

pub(super) const DISPLAY_SCALING: &[NamedValue] = &[
    NamedValue {
        name: "ratio-1-1",
        value: 1,
    },
    NamedValue {
        name: "ratio-full-scale-max",
        value: 2,
    },
    NamedValue {
        name: "ratio-full-scale-ratio",
        value: 5,
    },
    NamedValue {
        name: "ratio-17-4-3",
        value: 11,
    },
    NamedValue {
        name: "ratio-19-4-3",
        value: 12,
    },
    NamedValue {
        name: "ratio-19-5-4",
        value: 13,
    },
    NamedValue {
        name: "ratio-19w-16-10",
        value: 14,
    },
    NamedValue {
        name: "ratio-215w-16-9",
        value: 15,
    },
    NamedValue {
        name: "ratio-22w-16-10",
        value: 16,
    },
    NamedValue {
        name: "ratio-23w-16-9",
        value: 17,
    },
    NamedValue {
        name: "ratio-236-16-9",
        value: 18,
    },
    NamedValue {
        name: "ratio-24w-16-9",
        value: 19,
    },
    NamedValue {
        name: "ratio-27w-16-9",
        value: 20,
    },
    NamedValue {
        name: "ratio-30w-21-9",
        value: 21,
    },
    NamedValue {
        name: "ratio-32w-16-9",
        value: 22,
    },
    NamedValue {
        name: "ratio-34w-21-9",
        value: 23,
    },
    NamedValue {
        name: "ratio-40w-16-9",
        value: 24,
    },
    NamedValue {
        name: "ratio-42w-16-9",
        value: 25,
    },
    NamedValue {
        name: "ratio-movie1",
        value: 33,
    },
    NamedValue {
        name: "ratio-movie2",
        value: 34,
    },
    NamedValue {
        name: "ratio-aspect",
        value: 35,
    },
    NamedValue {
        name: "ratio-full-16-9",
        value: 39,
    },
    NamedValue {
        name: "ratio-1-1-16-9",
        value: 40,
    },
    NamedValue {
        name: "ratio-full-square",
        value: 41,
    },
    NamedValue {
        name: "ratio-1-1-square",
        value: 42,
    },
    NamedValue {
        name: "ratio-24-5",
        value: 43,
    },
    NamedValue {
        name: "ratio-27",
        value: 44,
    },
];

pub(super) const HDR_1: &[NamedValue] = &[
    NamedValue {
        name: "off",
        value: 0,
    },
    NamedValue {
        name: "auto",
        value: 1,
    },
];

pub(super) const HDR_3: &[NamedValue] = &[
    NamedValue {
        name: "off",
        value: 0,
    },
    NamedValue {
        name: "hdr-picture",
        value: 1,
    },
    NamedValue {
        name: "hdr-movie",
        value: 2,
    },
    NamedValue {
        name: "hdr-game",
        value: 3,
    },
];

pub(super) const HDR_4: &[NamedValue] = &[
    NamedValue {
        name: "off",
        value: 0,
    },
    NamedValue {
        name: "hdr-display",
        value: 1,
    },
    NamedValue {
        name: "hdr-picture",
        value: 2,
    },
    NamedValue {
        name: "hdr-movie",
        value: 3,
    },
    NamedValue {
        name: "hdr-game",
        value: 4,
    },
    NamedValue {
        name: "hdr-color",
        value: 5,
    },
    NamedValue {
        name: "hdr-peak",
        value: 6,
    },
];

pub(super) const LIGHT_FX_MODE_C: &[NamedValue] = &[
    NamedValue {
        name: "static",
        value: 0,
    },
    NamedValue {
        name: "simple-shift",
        value: 1,
    },
    NamedValue {
        name: "gradient-shift",
        value: 2,
    },
    NamedValue {
        name: "simple-filling",
        value: 3,
    },
    NamedValue {
        name: "1-way-filling",
        value: 4,
    },
    NamedValue {
        name: "2-way-filling",
        value: 5,
    },
    NamedValue {
        name: "breathing",
        value: 6,
    },
    NamedValue {
        name: "motion-point",
        value: 7,
    },
    NamedValue {
        name: "zoom",
        value: 8,
    },
    NamedValue {
        name: "discolor",
        value: 9,
    },
    NamedValue {
        name: "water-wave",
        value: 10,
    },
    NamedValue {
        name: "flashing",
        value: 11,
    },
    NamedValue {
        name: "demo",
        value: 12,
    },
];

pub(super) const LIGHT_FX_MODE_D: &[NamedValue] = &[
    NamedValue {
        name: "audio",
        value: 0,
    },
    NamedValue {
        name: "static",
        value: 1,
    },
    NamedValue {
        name: "simple-shift",
        value: 2,
    },
    NamedValue {
        name: "gradient-shift",
        value: 3,
    },
    NamedValue {
        name: "simple-filling",
        value: 4,
    },
    NamedValue {
        name: "1-way-filling",
        value: 5,
    },
    NamedValue {
        name: "2-way-filling",
        value: 6,
    },
    NamedValue {
        name: "breathing",
        value: 7,
    },
    NamedValue {
        name: "motion-point",
        value: 8,
    },
    NamedValue {
        name: "zoom",
        value: 9,
    },
    NamedValue {
        name: "discolor",
        value: 10,
    },
    NamedValue {
        name: "water-wave",
        value: 11,
    },
    NamedValue {
        name: "flashing",
        value: 12,
    },
    NamedValue {
        name: "demo",
        value: 13,
    },
];

pub(super) const LIGHT_FX_MODE_E: &[NamedValue] = &[
    NamedValue {
        name: "audio1",
        value: 0,
    },
    NamedValue {
        name: "audio2",
        value: 1,
    },
    NamedValue {
        name: "static",
        value: 2,
    },
    NamedValue {
        name: "dark-point-sweep",
        value: 3,
    },
    NamedValue {
        name: "gradient-shift",
        value: 4,
    },
    NamedValue {
        name: "spread-fill",
        value: 5,
    },
    NamedValue {
        name: "drip-fill",
        value: 6,
    },
    NamedValue {
        name: "spreading-drip-fill",
        value: 7,
    },
    NamedValue {
        name: "breathing",
        value: 8,
    },
    NamedValue {
        name: "light-point-sweep",
        value: 9,
    },
    NamedValue {
        name: "zoom",
        value: 10,
    },
    NamedValue {
        name: "rainbow",
        value: 11,
    },
    NamedValue {
        name: "wave",
        value: 12,
    },
    NamedValue {
        name: "flashing",
        value: 13,
    },
    NamedValue {
        name: "demo",
        value: 14,
    },
];

pub(super) const LIGHT_FX_MODE_F: &[NamedValue] = &[
    NamedValue {
        name: "lol",
        value: 0,
    },
    NamedValue {
        name: "audio1",
        value: 1,
    },
    NamedValue {
        name: "audio2",
        value: 2,
    },
    NamedValue {
        name: "static",
        value: 3,
    },
    NamedValue {
        name: "dark-point-sweep",
        value: 4,
    },
    NamedValue {
        name: "gradient-shift",
        value: 5,
    },
    NamedValue {
        name: "spread-fill",
        value: 6,
    },
    NamedValue {
        name: "drip-fill",
        value: 7,
    },
    NamedValue {
        name: "spreading-drip-fill",
        value: 8,
    },
    NamedValue {
        name: "breathing",
        value: 9,
    },
    NamedValue {
        name: "light-point-sweep",
        value: 10,
    },
    NamedValue {
        name: "zoom",
        value: 11,
    },
    NamedValue {
        name: "rainbow",
        value: 12,
    },
    NamedValue {
        name: "wave",
        value: 13,
    },
    NamedValue {
        name: "flashing",
        value: 14,
    },
    NamedValue {
        name: "demo",
        value: 15,
    },
];

pub(super) const LIGHT_FX_MODE_10: &[NamedValue] = &[
    NamedValue {
        name: "cs",
        value: 0,
    },
    NamedValue {
        name: "audio1",
        value: 1,
    },
    NamedValue {
        name: "audio2",
        value: 2,
    },
    NamedValue {
        name: "static",
        value: 3,
    },
    NamedValue {
        name: "dark-point-sweep",
        value: 4,
    },
    NamedValue {
        name: "gradient-shift",
        value: 5,
    },
    NamedValue {
        name: "spread-fill",
        value: 6,
    },
    NamedValue {
        name: "drip-fill",
        value: 7,
    },
    NamedValue {
        name: "spreading-drip-fill",
        value: 8,
    },
    NamedValue {
        name: "breathing",
        value: 9,
    },
    NamedValue {
        name: "light-point-sweep",
        value: 10,
    },
    NamedValue {
        name: "zoom",
        value: 11,
    },
    NamedValue {
        name: "rainbow",
        value: 12,
    },
    NamedValue {
        name: "wave",
        value: 13,
    },
    NamedValue {
        name: "flashing",
        value: 14,
    },
    NamedValue {
        name: "demo",
        value: 15,
    },
];

pub(super) const SWITCH_FLAG: &[NamedValue] = &[
    NamedValue {
        name: "off",
        value: 0,
    },
    NamedValue {
        name: "on",
        value: 1,
    },
];

const HDR_VARIANTS: &[NamedValueVariant] = &[
    NamedValueVariant {
        maximum: NamedValueMaximum::AtMost(1),
        values: HDR_1,
    },
    NamedValueVariant {
        maximum: NamedValueMaximum::Equal(3),
        values: HDR_3,
    },
    NamedValueVariant {
        maximum: NamedValueMaximum::Any,
        values: HDR_4,
    },
];

const LIGHT_FX_MODE_VARIANTS: &[NamedValueVariant] = &[
    NamedValueVariant {
        maximum: NamedValueMaximum::Equal(12),
        values: LIGHT_FX_MODE_C,
    },
    NamedValueVariant {
        maximum: NamedValueMaximum::Equal(13),
        values: LIGHT_FX_MODE_D,
    },
    NamedValueVariant {
        maximum: NamedValueMaximum::Equal(14),
        values: LIGHT_FX_MODE_E,
    },
    NamedValueVariant {
        maximum: NamedValueMaximum::Equal(15),
        values: LIGHT_FX_MODE_F,
    },
    NamedValueVariant {
        maximum: NamedValueMaximum::Equal(16),
        values: LIGHT_FX_MODE_10,
    },
];

impl NamedValueSelector {
    pub fn variants(self) -> &'static [NamedValueVariant] {
        match self {
            Self::Hdr => HDR_VARIANTS,
            Self::LightFxMode => LIGHT_FX_MODE_VARIANTS,
        }
    }

    pub(super) fn values(self, maximum: u16) -> &'static [NamedValue] {
        self.variants()
            .iter()
            .find(|variant| variant.maximum.matches(maximum))
            .map_or(&[], |variant| variant.values)
    }
}
