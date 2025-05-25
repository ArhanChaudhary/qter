use std::{
    cell::{Cell, Ref, RefCell},
    mem,
};

/// Information about each disjoint set and path as well as how to merge them together
///
/// The type that implements this trait is the type representing information for each set
///
/// The set info is metadata associated with each disjoint set in union-find.
///
/// The path info is info associated with each element of the union find, describing its path to the root.
pub trait SetInfo {
    /// The type representing information for each path
    type PathInfo;

    /// Whether to allow weighted quick-union (for better performance) or to force unions to happen in the order specified by the arguments to `union`
    const ALLOW_WEIGHTED: bool = false;

    /// Merge the info for two sets, used on the `union` call. Return the path info for the new child.
    fn merge(&mut self, new_child: Self);

    /// Join a path with the path of its parent. This function must be associative.
    fn join_paths(path: &mut Self::PathInfo, path_of_parent: &Self::PathInfo);
}

impl SetInfo for () {
    type PathInfo = ();

    const ALLOW_WEIGHTED: bool = true;

    fn merge(&mut self, _new_child: Self) {}

    fn join_paths(_path: &mut Self::PathInfo, _path_of_old_root: &Self::PathInfo) {}
}

enum UnionFindEntry<S: SetInfo> {
    RootOfSet {
        // For weighted union-find
        weight: usize,
        set_meta: S,
    },
    OwnedBy {
        owned_by: Cell<usize>,
    },
}

/// A data structure allowing you track disjoint sets of numbers. In qter, this means orbits in a permutation group but you can use it for anything.
///
/// This structure also keeps track of metadata for each set and element. If you do not need this, use `()` for the `S` parameter.
pub struct UnionFind<S: SetInfo> {
    sets: Box<[(UnionFindEntry<S>, RefCell<Option<S::PathInfo>>)]>,
}

/// Information about an element, returned by the `find` operation
pub struct FindResult<'a, S: SetInfo> {
    root_idx: usize,
    set_size: usize,
    set_meta: &'a S,
    path_meta: Ref<'a, Option<S::PathInfo>>,
}

impl<S: SetInfo> FindResult<'_, S> {
    /// Returns the index of the element representing the root of the set
    #[must_use]
    pub fn root_idx(&self) -> usize {
        self.root_idx
    }

    /// The total size of the set
    #[must_use]
    pub fn set_size(&self) -> usize {
        self.set_size
    }

    /// Metadata associated with the set the element is a member of
    #[must_use]
    pub fn set_meta(&self) -> &S {
        self.set_meta
    }

    /// Metadata associated with the path from this element to the root
    #[must_use]
    pub fn path_meta(&self) -> Option<&S::PathInfo> {
        self.path_meta.as_ref()
    }
}

impl<S: SetInfo + Default> UnionFind<S> {
    pub fn new(item_count: usize) -> Self {
        let mut sets = Vec::with_capacity(item_count);

        for _ in 0..item_count {
            sets.push((
                UnionFindEntry::RootOfSet {
                    weight: 1,
                    set_meta: S::default(),
                },
                RefCell::new(None),
            ));
        }

        UnionFind {
            sets: Box::from(sets),
        }
    }
}

impl<S: SetInfo> UnionFind<S> {
    /// Create a new `UnionFind` with the given number of elements
    pub fn new_with_initial_set_info(set_infos: Vec<S>) -> Self {
        let mut sets = Vec::with_capacity(set_infos.len());

        for info in set_infos {
            sets.push((
                UnionFindEntry::RootOfSet {
                    weight: 1,
                    set_meta: info,
                },
                RefCell::new(None),
            ));
        }

        UnionFind {
            sets: Box::from(sets),
        }
    }

    /// Find an element in the `UnionFind` and return metadata about it.
    ///
    /// Panics if the item is outside the range of numbers in the union-find.
    #[must_use]
    #[expect(clippy::missing_panics_doc)]
    pub fn find(&self, item: usize) -> FindResult<S> {
        let (entry, path_meta) = &self.sets[item];

        match entry {
            UnionFindEntry::RootOfSet { weight, set_meta } => FindResult {
                root_idx: item,
                set_size: *weight,
                set_meta,
                path_meta: path_meta.borrow(),
            },
            UnionFindEntry::OwnedBy { owned_by } => {
                let mut ret = self.find(owned_by.get());

                if let Some(root_meta) = ret.path_meta() {
                    owned_by.set(ret.root_idx);

                    // This borrow_mut is valid despite a `Ref` being returned because once this function returns, the node is guaranteed to be a child of a root and compression will not happen again until `union` is called. This function returns an `&` reference to the union-find and `union` takes and `&mut` reference so `union` will not be called until the `Ref` is dropped.
                    let mut path_meta_mut = path_meta.borrow_mut();
                    // This element has a parent, therefore the `path_meta` cannot be null
                    S::join_paths((*path_meta_mut).as_mut().unwrap(), root_meta);
                    drop(path_meta_mut);
                    ret.path_meta = path_meta.borrow();
                }

                ret
            }
        }
    }

    /// Union the sets that the two representatives given belong to, with `child` becoming a child of `parent`.
    ///
    /// Panics if either `parent` or `child` are outside of the range of elements in the union-find.
    ///
    /// If `S::ALLOW_WEIGHTED` is `true`, then this will implement weighted quick union and `parent` and `child` may be swapped for performance.
    pub fn union(&mut self, parent: usize, child: usize, path_info: S::PathInfo) {
        let mut a_result = self.find(parent);
        let mut b_result = self.find(child);

        if a_result.root_idx == b_result.root_idx {
            return;
        }

        if S::ALLOW_WEIGHTED && a_result.set_size < b_result.set_size {
            mem::swap(&mut a_result, &mut b_result);
        }

        let a_idx = a_result.root_idx;
        let b_size = b_result.set_size;
        let b_idx = b_result.root_idx;

        drop(a_result);
        drop(b_result);

        self.sets[b_idx].1.replace(Some(path_info));

        let old_b_data = mem::replace(
            &mut self.sets[b_idx].0,
            UnionFindEntry::OwnedBy {
                owned_by: Cell::new(a_idx),
            },
        );

        let other_set_meta = match old_b_data {
            UnionFindEntry::RootOfSet {
                weight: _,
                set_meta,
            } => set_meta,
            UnionFindEntry::OwnedBy { owned_by: _ } => unreachable!(),
        };

        match &mut self.sets[a_idx].0 {
            UnionFindEntry::RootOfSet { weight, set_meta } => {
                *weight += b_size;

                set_meta.merge(other_set_meta);
            }
            UnionFindEntry::OwnedBy { owned_by: _ } => unreachable!(),
        }
    }
}
