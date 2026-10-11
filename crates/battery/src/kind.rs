//! What kind of device something is (mouse, keyboard, ...), used to pick the
//! default icon and the default tile order.

use std::collections::HashSet;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Declaration order is the default tile order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceKind {
    Laptop,
    Mouse,
    Keyboard,
    Headset,
    Gamepad,
    Other,
}

pub const ALL_KINDS: [DeviceKind; 6] = [
    DeviceKind::Laptop,
    DeviceKind::Mouse,
    DeviceKind::Keyboard,
    DeviceKind::Headset,
    DeviceKind::Gamepad,
    DeviceKind::Other,
];

impl DeviceKind {
    /// Returns the lowercase name used in the config file and the CLI.
    pub fn as_str(self) -> &'static str {
        match self {
            DeviceKind::Laptop => "laptop",
            DeviceKind::Mouse => "mouse",
            DeviceKind::Keyboard => "keyboard",
            DeviceKind::Headset => "headset",
            DeviceKind::Gamepad => "gamepad",
            DeviceKind::Other => "other",
        }
    }
}

impl fmt::Display for DeviceKind {
    /// Prints the same lowercase name as in the config file.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(self.as_str())
    }
}

impl FromStr for DeviceKind {
    type Err = String;

    /// Parses a kind name typed by the user, case-insensitive.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let wanted = s.trim().to_ascii_lowercase();
        ALL_KINDS
            .into_iter()
            .find(|k| k.as_str() == wanted)
            .ok_or_else(|| {
                let names: Vec<_> = ALL_KINDS.iter().map(|k| k.as_str()).collect();
                format!("unknown kind '{s}', expected one of: {}", names.join(", "))
            })
    }
}

const MOUSE_WORDS: &[&str] = &[
    "mouse",
    "viper",
    "deathadder",
    "basilisk",
    "naga",
    "cobra",
    "orochi",
    "mamba",
    "pro click",
    "mx master",
    "mx anywhere",
    "g pro",
    "superlight",
    "trackball",
];
const KEYBOARD_WORDS: &[&str] = &[
    "keyboard",
    "yunzii",
    "keychron",
    "blackwidow",
    "huntsman",
    "deathstalker",
    "ornata",
    "pro type",
    "mx keys",
    "nuphy",
];
const HEADSET_WORDS: &[&str] = &[
    "headset",
    "headphone",
    "buds",
    "airpods",
    "kraken",
    "blackshark",
    "barracuda",
    "hammerhead",
    "nari",
    "wh-",
    "wf-",
    "earbuds",
    "hands-free",
];
const GAMEPAD_WORDS: &[&str] = &[
    "controller",
    "gamepad",
    "xbox",
    "dualsense",
    "dualshock",
    "joy-con",
    "8bitdo",
];

/// Guesses the kind from words in the device name; `Other` if nothing matches.
pub fn guess_kind_from_name(name: &str) -> DeviceKind {
    let lower = name.to_lowercase();
    let groups = [
        (DeviceKind::Mouse, MOUSE_WORDS),
        (DeviceKind::Keyboard, KEYBOARD_WORDS),
        (DeviceKind::Headset, HEADSET_WORDS),
        (DeviceKind::Gamepad, GAMEPAD_WORDS),
    ];
    for (kind, words) in groups {
        if words.iter().any(|w| lower.contains(w)) {
            return kind;
        }
    }
    DeviceKind::Other
}

/// Guesses the kind from the Windows device classes ("Mouse", "Keyboard",
/// "AudioEndpoint", ...) of the nodes that belong to one physical device.
/// Returns `None` when the classes are missing or contradict each other —
/// many mice also expose a keyboard interface and vice versa.
pub fn guess_kind_from_classes(classes: &HashSet<String>) -> Option<DeviceKind> {
    let has = |name: &str| classes.iter().any(|c| c.eq_ignore_ascii_case(name));
    let mouse = has("Mouse");
    let keyboard = has("Keyboard");
    if has("XnaComposite") || has("XboxComposite") {
        return Some(DeviceKind::Gamepad);
    }
    if mouse && !keyboard {
        return Some(DeviceKind::Mouse);
    }
    if keyboard && !mouse {
        return Some(DeviceKind::Keyboard);
    }
    if !mouse && !keyboard && (has("AudioEndpoint") || has("MEDIA")) {
        return Some(DeviceKind::Headset);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Checks that known product names map to the right kind.
    #[test]
    fn guesses_kind_from_product_names() {
        assert_eq!(
            guess_kind_from_name("Razer Viper V3 Pro SE"),
            DeviceKind::Mouse
        );
        assert_eq!(
            guess_kind_from_name("YUNZII B75PROMAX"),
            DeviceKind::Keyboard
        );
        assert_eq!(
            guess_kind_from_name("Galaxy Buds2 Pro"),
            DeviceKind::Headset
        );
        assert_eq!(
            guess_kind_from_name("Xbox Wireless Controller"),
            DeviceKind::Gamepad
        );
        assert_eq!(guess_kind_from_name("Pixel 9"), DeviceKind::Other);
    }

    /// Checks that a mouse with a macro keyboard interface stays undecided.
    #[test]
    fn mixed_mouse_and_keyboard_classes_are_undecided() {
        let classes: HashSet<String> = ["Mouse", "Keyboard", "HIDClass"]
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(guess_kind_from_classes(&classes), None);
    }

    /// Checks that kind names round-trip through text.
    #[test]
    fn parses_kind_names() {
        assert_eq!("Mouse".parse::<DeviceKind>(), Ok(DeviceKind::Mouse));
        assert!("toaster".parse::<DeviceKind>().is_err());
    }
}
