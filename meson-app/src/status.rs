//! Status → Orbital badge appearance for File virus-scan states.

use uf_product::primitives::{BadgeAppearance, BadgeColor};

/// Map a File `file_status` wire string to badge label, appearance, and color.
#[must_use]
pub fn badge_for_file_status(status: &str) -> (String, BadgeAppearance, BadgeColor) {
    match status {
        "available" => (
            "Available".to_string(),
            BadgeAppearance::Filled,
            BadgeColor::Success,
        ),
        "pending_virus_scan" | "virus_scan_complete" => (
            "Pending".to_string(),
            BadgeAppearance::Tint,
            BadgeColor::Informative,
        ),
        "quarantined" => (
            "Quarantined".to_string(),
            BadgeAppearance::Filled,
            BadgeColor::Danger,
        ),
        other => (
            other.to_string(),
            BadgeAppearance::Outline,
            BadgeColor::Subtle,
        ),
    }
}
