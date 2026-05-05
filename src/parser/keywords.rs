use std::collections::HashSet;
use std::sync::LazyLock;

pub(super) static ATL_WARPERS: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(include!("./atl_warpers.inc")));
pub(super) static ATL_PROPERTIES: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(include!("./atl_properties.inc")));
pub(super) static STYLE_PROPERTIES: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(include!("./style_properties.inc")));
