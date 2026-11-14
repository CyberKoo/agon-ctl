use crate::Feature;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SupportPath {
    CapabilityList,
    Maximum,
}

impl SupportPath {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CapabilityList => "capability-list",
            Self::Maximum => "maximum",
        }
    }
}

impl Feature {
    pub const fn support_path(self) -> Option<SupportPath> {
        match self {
            Self::ColorPreset
            | Self::InputSource
            | Self::DisplayScaling
            | Self::GameMode
            | Self::Gamma
            | Self::Overdrive
            | Self::ColorGamut
            | Self::LocalDimming
            | Self::ShadowBoost
            | Self::SharpShooterSize
            | Self::PixelOrbiting
            | Self::ScreenSaver
            | Self::MultiLogoProtection
            | Self::BoundaryDimmer
            | Self::TaskbarDimmer
            | Self::AdaptiveSync
            | Self::AutoWarning
            | Self::Lea
            | Self::ThermalProtection
            | Self::AutoPixelRefresh
            | Self::RaceAssist
            | Self::Spotlight
            | Self::NightSight
            | Self::ClearSight
            | Self::LifeGlow
            | Self::AutoVision
            | Self::AudioOutput
            | Self::DaisyChain => Some(SupportPath::CapabilityList),
            Self::HdrMode | Self::LightFxMode | Self::DialPoint => Some(SupportPath::Maximum),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn support_paths_match_the_per_feature_report_classification() {
        const PATH_A: &[Feature] = &[
            Feature::ColorPreset,
            Feature::InputSource,
            Feature::DisplayScaling,
            Feature::GameMode,
            Feature::Gamma,
            Feature::Overdrive,
            Feature::ColorGamut,
            Feature::LocalDimming,
            Feature::ShadowBoost,
            Feature::SharpShooterSize,
            Feature::PixelOrbiting,
            Feature::ScreenSaver,
            Feature::MultiLogoProtection,
            Feature::BoundaryDimmer,
            Feature::TaskbarDimmer,
            Feature::AdaptiveSync,
            Feature::AutoWarning,
            Feature::Lea,
            Feature::ThermalProtection,
            Feature::AutoPixelRefresh,
            Feature::RaceAssist,
            Feature::Spotlight,
            Feature::NightSight,
            Feature::ClearSight,
            Feature::LifeGlow,
            Feature::AutoVision,
            Feature::AudioOutput,
            Feature::DaisyChain,
        ];
        const PATH_B: &[Feature] = &[Feature::HdrMode, Feature::LightFxMode, Feature::DialPoint];

        for feature in Feature::ALL.iter().copied() {
            let expected = if PATH_A.contains(&feature) {
                Some(SupportPath::CapabilityList)
            } else if PATH_B.contains(&feature) {
                Some(SupportPath::Maximum)
            } else {
                None
            };
            assert_eq!(feature.support_path(), expected, "{feature}");
        }
        assert_eq!(PATH_A.len(), 28);
        assert_eq!(SupportPath::CapabilityList.as_str(), "capability-list");
        assert_eq!(SupportPath::Maximum.as_str(), "maximum");
    }
}
