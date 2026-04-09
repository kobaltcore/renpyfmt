use lazy_static::lazy_static;
use std::collections::HashSet;

lazy_static! {
    pub(super) static ref ATL_WARPERS: HashSet<&'static str> =
        HashSet::from(include!("./atl_warpers.inc"));
    pub(super) static ref ATL_PROPERTIES: HashSet<&'static str> =
        HashSet::from(include!("./atl_properties.inc"));
    pub(super) static ref STYLE_PROPERTIES: HashSet<&'static str> =
        HashSet::from(include!("./style_properties.inc"));
}
