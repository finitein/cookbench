use std::collections::HashSet;

use cookbench_adapters::{catalog, harness_profile, HookDialect, ReturnSurface, SupportTier};

#[test]
fn catalog_exceeds_code_island_without_duplicate_or_unsafe_ids() {
    let profiles = catalog();
    assert_eq!(profiles.len(), 27);
    assert!(profiles.len() > 14);

    let ids = profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<HashSet<_>>();
    assert_eq!(ids.len(), profiles.len());
    assert!(profiles.iter().all(|profile| {
        !profile.label.is_empty()
            && profile.id.len() <= 32
            && profile
                .id
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
            && profile.reference.starts_with("https://")
    }));
}

#[test]
fn requested_chinese_and_existing_harnesses_are_first_class_profiles() {
    for id in [
        "codex",
        "claude_code",
        "pi",
        "zcode",
        "workbuddy",
        "qoder",
        "kimi_code",
        "qwen_code",
        "codebuddy",
    ] {
        assert!(harness_profile(id).is_some(), "missing {id}");
    }
}

#[test]
fn full_support_requires_structured_hooks_and_a_verified_return_surface() {
    for profile in catalog()
        .iter()
        .filter(|profile| profile.tier == SupportTier::Full)
    {
        assert_ne!(profile.hook_dialect, HookDialect::None, "{}", profile.id);
        assert_ne!(
            profile.return_surface,
            ReturnSurface::PresenceOnly,
            "{}",
            profile.id
        );
        assert!(profile.structured_lifecycle, "{}", profile.id);
    }
}

#[test]
fn experimental_support_never_claims_structured_completion() {
    let workbuddy = harness_profile("workbuddy").unwrap();
    assert_eq!(workbuddy.tier, SupportTier::Experimental);
    assert_eq!(workbuddy.hook_dialect, HookDialect::None);
    assert_eq!(workbuddy.return_surface, ReturnSurface::PresenceOnly);
    assert!(!workbuddy.structured_lifecycle);
}

#[test]
fn process_and_root_metadata_are_bounded_and_content_free() {
    for profile in catalog() {
        assert!(profile.executables.len() <= 8, "{}", profile.id);
        assert!(profile.default_roots.len() <= 8, "{}", profile.id);
        assert!(profile
            .executables
            .iter()
            .all(|value| !value.is_empty() && value.len() <= 64));
        assert!(profile
            .default_roots
            .iter()
            .all(|value| !value.is_empty() && value.len() <= 256));
    }
}
