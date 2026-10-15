use petunia_config::keybinds::{Keybinds, Mods2, parse_binding, winit_keys::KeyCode};

#[test]
fn punctuation_and_numpad_plus_roundtrip() {
    for key in [
        "[",
        "]",
        "Shift+[",
        "Shift+]",
        "NumpadAdd",
        "Ctrl+Numpad +",
        "Backslash",
        "Quote",
    ] {
        let binding = parse_binding(key).expect(key);
        assert_eq!(
            parse_binding(&binding.to_shortcut_string()),
            Some(binding),
            "{key}"
        );
    }
    for invalid in ["", "Ctrl+", "Ctrl+Bogus+A", "A+B", "Ctrl+Ctrl+A"] {
        assert!(parse_binding(invalid).is_none(), "{invalid}");
    }
}

#[test]
fn removed_bindings_and_escaped_keys_survive_saved_profile() {
    let dir = std::env::temp_dir().join(format!("petunia-keymap-integrity-{}", std::process::id()));
    let mut kb = Keybinds::defaults();
    kb.remove_binding("model.extrude");
    kb.set_binding("model.custom", parse_binding("Backslash").unwrap());
    let id = kb
        .save_user_profile_in(&dir, "Round trip", "Synthetic regression")
        .unwrap();
    let reloaded = Keybinds::load_profile_in(&id, &dir);
    assert!(reloaded.binding_for("model.extrude").is_none());
    assert_eq!(
        reloaded.binding_for("model.custom"),
        kb.binding_for("model.custom")
    );
    kb.set_binding("model.extrude", parse_binding("E").unwrap());
    let id = kb
        .save_user_profile_in(&dir, "Round trip", "Re-enabled")
        .unwrap();
    assert!(
        Keybinds::load_profile_in(&id, &dir)
            .binding_for("model.extrude")
            .is_some()
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn brush_defaults_and_maya_move_are_reachable() {
    let kb = Keybinds::load_profile("petunia-default");
    for id in [
        "paint.size_decrease",
        "paint.size_increase",
        "paint.hardness_decrease",
        "paint.hardness_increase",
    ] {
        assert!(kb.binding_for(id).is_some(), "{id}");
    }
    let maya = Keybinds::load_profile("maya");
    assert_eq!(
        maya.find_in_context(KeyCode::KeyW, Mods2::default(), "model"),
        Some("model.transform")
    );
}

#[test]
fn conflict_detectors_agree_for_all_shared_contexts() {
    for shared in ["global", "view", "window"] {
        let mut kb = Keybinds::default();
        let binding = parse_binding("F").unwrap();
        kb.set_binding(format!("{shared}.test"), binding.clone());
        kb.set_binding("model.test", binding.clone());
        assert_eq!(kb.conflicts_for("model.test", &binding).len(), 1);
        assert_eq!(kb.detect_conflicts().len(), 1, "{shared}");
    }
}

#[test]
fn canonical_profiles_have_no_same_context_collisions_and_custom_ambiguity_is_explicit() {
    for profile in Keybinds::available_profiles() {
        let kb = Keybinds::load_profile(&profile.id);
        let exact: Vec<_> = kb
            .detect_conflicts()
            .into_iter()
            .filter(|c| c.kind == petunia_config::keybinds::ConflictKind::Exact)
            .collect();
        assert!(exact.is_empty(), "{}: {exact:?}", profile.id);
    }
    let mut kb = Keybinds::default();
    kb.set_binding("model.first", parse_binding("F").unwrap());
    kb.set_binding("model.second", parse_binding("F").unwrap());
    assert_eq!(
        kb.find_in_context(KeyCode::KeyF, Mods2::default(), "model"),
        None
    );
}
