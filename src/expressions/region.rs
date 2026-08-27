//! Capture slots (Oniguruma OnigRegion). Offsets are haystack byte offsets.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::ops::Range;

/// One match: whole-match range plus numbered (and optional named) groups.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Region {
    /// Group 0 is the whole match. Unset groups are `None`.
    pub captures: Vec<Option<Range<usize>>>,
    /// Parallel names: `names[i]` is Some for a named group number i.
    ///
    /// **Empty** when the pattern has no named groups at all -- filling it
    /// with `None`s would allocate for nothing. Read it through
    /// [`Region::name`], or bound any direct indexing by `names.len()`, which
    /// is either 0 or `captures.len()`.
    pub names: Vec<Option<String>>,
    /// Capture-history tree (`onig_capture_tree_traverse`) when `(?@...)` ran.
    pub history: Option<CaptureTree>,
}

/// One node in the capture-history tree (`OnigCaptureTreeNode`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureTree {
    pub group: usize,
    pub range: Range<usize>,
    pub children: Vec<CaptureTree>,
}

impl CaptureTree {
    /// Depth-first walk: `f(node, depth)`.
    pub fn traverse<F: FnMut(&CaptureTree, usize)>(&self, mut f: F) {
        fn walk<F: FnMut(&CaptureTree, usize)>(n: &CaptureTree, d: usize, f: &mut F) {
            f(n, d);
            for c in &n.children {
                walk(c, d + 1, f);
            }
        }
        walk(self, 0, &mut f);
    }
}

impl Region {
    /// `named == false` skips the names vector entirely; `name()` then finds
    /// nothing, which is correct for a pattern that has no named groups.
    pub(crate) fn with_names(n: usize, named: bool) -> Self {
        Self {
            captures: alloc::vec![None; n],
            names: if named {
                alloc::vec![None; n]
            } else {
                Vec::new()
            },
            history: None,
        }
    }

    /// Whole-match byte range (group 0).
    pub fn range(&self) -> Range<usize> {
        self.captures
            .first()
            .and_then(|c| c.clone())
            .unwrap_or(0..0)
    }

    /// Numbered group, 0 = whole match.
    pub fn get(&self, i: usize) -> Option<Range<usize>> {
        self.captures.get(i).and_then(|c| c.clone())
    }

    /// First group with this name.
    pub fn name(&self, name: &str) -> Option<Range<usize>> {
        for (i, n) in self.names.iter().enumerate() {
            if n.as_deref() == Some(name) {
                return self.get(i);
            }
        }
        None
    }

    pub fn is_empty_match(&self) -> bool {
        let r = self.range();
        r.start == r.end
    }

    /// Walk the capture-history tree (`onig_capture_tree_traverse`).
    pub fn traverse_history<F: FnMut(&CaptureTree, usize)>(&self, f: F) {
        if let Some(t) = &self.history {
            t.traverse(f);
        }
    }
}
