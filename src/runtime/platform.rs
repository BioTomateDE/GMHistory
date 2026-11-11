use strum::{EnumString, IntoStaticStr, VariantArray};

/// Corresponds to the "name" field in the XML tag of the RSS manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumString, IntoStaticStr, VariantArray)]
pub enum Platform {
    #[strum(serialize = "amazonfire")]
    AmazonFire,

    #[strum(serialize = "android")]
    Android,

    #[strum(serialize = "base-module-linux-arm")]
    BaseModuleLinuxArm,

    #[strum(serialize = "base-module-linux-arm64")]
    BaseModuleLinuxArm64,

    #[strum(serialize = "base-module-linux-x64")]
    BaseModuleLinuxX64,

    #[strum(serialize = "base-module-osx-arm64")]
    BaseModuleOsxArm64,

    #[strum(serialize = "base-module-osx-x64")]
    BaseModuleOsxX64,

    #[strum(serialize = "base-module-windows-x64")]
    BaseModuleWindowsX64,

    #[strum(serialize = "html5")]
    Html5,

    #[strum(serialize = "ios")]
    Ios,

    #[strum(serialize = "linux")]
    Linux,

    #[strum(serialize = "linuxYYC")]
    LinuxYYC,

    #[strum(serialize = "mac")]
    Mac,

    #[strum(serialize = "mac.mono")]
    MacMono,

    #[strum(serialize = "macYYC")]
    MacYYC,

    #[strum(serialize = "operagx")]
    OperaGx,

    #[strum(serialize = "operagxYYC")]
    OperaGxYYC,

    #[strum(serialize = "ps4")]
    Ps4,

    #[strum(serialize = "ps5")]
    Ps5,

    #[strum(serialize = "switch")]
    Switch,

    #[strum(serialize = "switch2")]
    Switch2,

    #[strum(serialize = "tvos")]
    TvOs,

    #[strum(serialize = "windows")]
    Windows,

    #[strum(serialize = "windowsuap")]
    WindowsUap,

    #[strum(serialize = "windowsYYC")]
    WindowsYYC,

    #[strum(serialize = "xboxone")]
    XboxOne,

    #[strum(serialize = "xboxseriesxs")]
    XboxSeriesXs,
}
