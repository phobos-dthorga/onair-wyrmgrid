use super::{StartupOptions, parse_startup_options};

fn parse(arguments: &[&str]) -> StartupOptions {
    parse_startup_options(arguments)
}

#[test]
fn uses_full_presentation_by_default() {
    let options = parse(&[]);

    assert!(!options.no_launch_art);
    assert!(!options.compact_ui);
    assert!(!options.low_resource);
}

#[test]
fn keeps_launch_art_and_compact_layout_independent() {
    let no_art = parse(&["--no-launch-art"]);
    assert!(no_art.no_launch_art);
    assert!(!no_art.compact_ui);
    assert!(!no_art.low_resource);

    let compact = parse(&["--compact-ui"]);
    assert!(!compact.no_launch_art);
    assert!(compact.compact_ui);
    assert!(!compact.low_resource);
}

#[test]
fn low_resource_mode_implies_both_presentation_overrides() {
    let options = parse(&["--low-resource"]);

    assert!(options.no_launch_art);
    assert!(options.compact_ui);
    assert!(options.low_resource);
}

#[test]
fn ignores_unrelated_process_arguments() {
    let options = parse(&["--enable-logging", "document.wyrmgrid"]);

    assert!(!options.no_launch_art);
    assert!(!options.compact_ui);
    assert!(!options.low_resource);
}
