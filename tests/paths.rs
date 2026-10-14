use std::{collections::HashMap, ffi::OsString, path::PathBuf};

use alba::paths::Paths;

fn resolve(variables: &[(&str, &str)]) -> Option<Paths> {
    let variables: HashMap<_, _> = variables.iter().copied().collect();
    Paths::resolve(|name| variables.get(name).map(OsString::from)).ok()
}

#[test]
fn unset_xdg_variables_fall_back_to_their_defaults_under_home() {
    let paths = resolve(&[("HOME", "/home/ana")]).unwrap();

    assert_eq!(
        paths.config_file,
        PathBuf::from("/home/ana/.config/alba/config.toml")
    );
    assert_eq!(
        paths.cache_directory,
        PathBuf::from("/home/ana/.cache/alba")
    );
    assert_eq!(
        paths.state_directory,
        PathBuf::from("/home/ana/.local/state/alba")
    );
}

#[test]
fn xdg_variables_win_over_the_defaults() {
    let paths = resolve(&[
        ("HOME", "/home/ana"),
        ("XDG_CONFIG_HOME", "/etc/ana"),
        ("XDG_CACHE_HOME", "/tmp/cache"),
        ("XDG_STATE_HOME", "/var/ana"),
    ])
    .unwrap();

    assert_eq!(
        paths.config_file,
        PathBuf::from("/etc/ana/alba/config.toml")
    );
    assert_eq!(paths.cache_directory, PathBuf::from("/tmp/cache/alba"));
    assert_eq!(paths.state_directory, PathBuf::from("/var/ana/alba"));
}

#[test]
fn empty_or_relative_xdg_variables_are_ignored() {
    let paths = resolve(&[
        ("HOME", "/home/ana"),
        ("XDG_CONFIG_HOME", ""),
        ("XDG_CACHE_HOME", "cache"),
    ])
    .unwrap();

    assert_eq!(
        paths.config_file,
        PathBuf::from("/home/ana/.config/alba/config.toml")
    );
    assert_eq!(
        paths.cache_directory,
        PathBuf::from("/home/ana/.cache/alba")
    );
}

#[test]
fn a_default_needs_home() {
    assert!(resolve(&[]).is_none());
    assert!(resolve(&[("HOME", "")]).is_none());
    assert!(resolve(&[("XDG_CONFIG_HOME", "/etc/ana")]).is_none());
}

#[test]
fn every_xdg_variable_set_needs_no_home() {
    let paths = resolve(&[
        ("XDG_CONFIG_HOME", "/etc/ana"),
        ("XDG_CACHE_HOME", "/tmp/cache"),
        ("XDG_STATE_HOME", "/var/ana"),
    ]);

    assert!(paths.is_some());
}
