use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub enum Comment {
    Standalone {
        indent: usize,
        text: String,
        line_number: usize,
    },
    Trailing {
        text: String,
        line_number: usize,
    },
}

pub type CommentMap = BTreeMap<usize, Vec<Comment>>;

pub const EOF_LINE: usize = usize::MAX;
