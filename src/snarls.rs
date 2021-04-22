use log::{debug, trace};

use rustc_hash::{FxHashMap, FxHashSet};

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Biedged {}
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cactus {}
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bridge {}

pub trait GraphType {}

impl GraphType for Biedged {}
impl GraphType for Cactus {}
impl GraphType for Bridge {}

/// A node index for a biedged graph of the specified type
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Node {
    pub id: u64,
}

impl From<u64> for Node {
    #[inline]
    fn from(id: u64) -> Self {
        Self { id }
    }
}

impl Node {
    #[inline]
    pub fn new(id: u64) -> Self {
        Node { id }
    }

    #[inline]
    pub fn id_mut(&mut self) -> &mut u64 {
        &mut self.id
    }

    /// Derive the node IDs for a black edge in a biedged graph, given
    /// a node ID in a GFA graph
    #[inline]
    pub fn from_gfa_id(id: u64) -> (Self, Self) {
        let left = id * 2;
        let right = left + 1;

        (Self::new(left), Self::new(right))
    }

    /// Derive the original GFA ID for the provided black edge node ID
    #[inline]
    pub fn to_gfa_id(&self) -> u64 {
        self.id / 2
    }

    /// Derive the pair of node IDs in the black edge defined by this node
    #[inline]
    pub fn black_edge(&self) -> (Self, Self) {
        let left = self.id & !1;
        let right = left + 1;

        (Self::new(left), Self::new(right))
    }

    /// Return the left-hand side of the node
    pub fn left(&self) -> Self {
        Self { id: self.id & !1 }
    }

    /// Return the right-hand side of the node
    pub fn right(&self) -> Self {
        Self { id: self.id | 1 }
    }

    /// Return the opposite node
    #[inline]
    pub fn opposite(&self) -> Self {
        Self { id: self.id ^ 1 }
    }

    #[inline]
    pub fn is_left(&self) -> bool {
        self.id & 1 == 0
    }

    #[inline]
    pub fn is_right(&self) -> bool {
        self.id & 1 != 0
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SnarlType {
    ChainPair,
    BridgePair,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Snarl<T: Copy + Eq + Ord + std::hash::Hash> {
    pub left: Node,
    pub right: Node,
    pub ty: SnarlType,
    data: T,
}

impl<T> Snarl<T>
where
    T: Default + Copy + Eq + Ord + std::hash::Hash,
{
    pub fn chain_pair(x: Node, y: Node) -> Self {
        let left = x.min(y);
        let right = x.max(y);

        Snarl {
            left,
            right,
            ty: SnarlType::ChainPair,
            data: T::default(),
        }
    }

    pub fn bridge_pair(x: Node, y: Node) -> Self {
        let left = x.min(y);
        let right = x.max(y);

        Snarl {
            left,
            right,
            ty: SnarlType::BridgePair,
            data: T::default(),
        }
    }
}

impl<T> Snarl<T>
where
    T: Copy + Eq + Ord + std::hash::Hash,
{
    pub fn left(&self) -> Node {
        self.left
    }

    pub fn right(&self) -> Node {
        self.right
    }

    pub fn snarl_type(&self) -> SnarlType {
        self.ty
    }

    pub fn data(&self) -> T {
        self.data
    }

    pub fn chain_pair_with(x: Node, y: Node, data: T) -> Self {
        let left = x.min(y);
        let right = x.max(y);

        Snarl {
            left,
            right,
            ty: SnarlType::ChainPair,
            data,
        }
    }

    pub fn bridge_pair_with(x: Node, y: Node, data: T) -> Self {
        let left = x.min(y);
        let right = x.max(y);

        Snarl {
            left,
            right,
            ty: SnarlType::BridgePair,
            data,
        }
    }

    pub fn map_data<F, U>(&self, f: F) -> Snarl<U>
    where
        F: Fn(T) -> U,
        U: Copy + Eq + Ord + std::hash::Hash,
    {
        Snarl {
            left: self.left,
            right: self.right,
            ty: self.ty,
            data: f(self.data),
        }
    }
}

#[derive(Default, Clone)]
pub struct SnarlMap {
    // Snarls indexed by left boundary
    pub lefts: FxHashMap<Node, Vec<usize>>,
    // Snarls indexed by right boundary
    pub rights: FxHashMap<Node, Vec<usize>>,

    // Snarls by rank
    pub snarls: FxHashMap<usize, Snarl<()>>,

    // Map of contained/not contained black edges for each snarl by rank
    pub snarl_contains: FxHashMap<usize, FxHashMap<Node, bool>>,
}

pub struct SnarlTree {
    map: SnarlMap,

    tree: FxHashMap<usize, FxHashSet<usize>>,
}

impl SnarlTree {
    pub fn from_snarl_map(snarl_map: SnarlMap) -> Self {
        let mut contains_by_size: Vec<(usize, FxHashSet<Node>)> = snarl_map
            .snarl_contains
            .iter()
            .map(|(&k, v)| {
                let bridges = v
                    .iter()
                    .filter_map(
                        |(&b, &contains)| {
                            if contains {
                                Some(b)
                            } else {
                                None
                            }
                        },
                    )
                    .collect::<FxHashSet<_>>();

                (k, bridges)
            })
            .collect();

        contains_by_size.sort_by_key(|(_, bridges)| bridges.len());

        let mut bridge_snarls: FxHashMap<Node, FxHashSet<usize>> =
            Default::default();

        for (&snarl_ix, contained) in snarl_map.snarl_contains.iter() {
            for (&bridge, &contains) in contained.iter() {
                if contains {
                    bridge_snarls.entry(bridge).or_default().insert(snarl_ix);
                }
            }
        }

        let mut tree: FxHashMap<usize, FxHashSet<usize>> = Default::default();

        Self {
            map: snarl_map,
            tree,
        }
    }
}

pub struct SnarlMapIter<'a> {
    lefts: Option<std::slice::Iter<'a, usize>>,
    rights: Option<std::slice::Iter<'a, usize>>,

    snarls: &'a FxHashMap<usize, Snarl<()>>,
}

impl<'a> SnarlMapIter<'a> {
    fn new(snarl_map: &'a SnarlMap, x: Node) -> Self {
        let lefts = snarl_map.lefts.get(&x).map(|lefts| lefts.iter());
        let rights = snarl_map.rights.get(&x).map(|rights| rights.iter());

        Self {
            lefts,
            rights,

            snarls: &snarl_map.snarls,
        }
    }
}

impl<'a> Iterator for SnarlMapIter<'a> {
    type Item = Snarl<()>;

    fn next(&mut self) -> Option<Snarl<()>> {
        if self.lefts.is_none() && self.rights.is_none() {
            return None;
        }

        if let Some(lefts) = self.lefts.as_mut() {
            if let Some(ix) = lefts.next() {
                let v = *self.snarls.get(ix)?;
                return Some(v);
            } else {
                self.lefts = None;
            }
        }

        if let Some(rights) = self.rights.as_mut() {
            if let Some(ix) = rights.next() {
                let v = *self.snarls.get(ix)?;
                return Some(v);
            } else {
                self.rights = None;
            }
        }

        None
    }
}

impl SnarlMap {
    pub fn insert(&mut self, snarl: Snarl<()>) {
        if self.get_snarl_ix(snarl.left, snarl.right).is_some() {
            return;
        }

        let ix = self.snarls.len();

        self.snarls.insert(ix, snarl);

        self.lefts.entry(snarl.left()).or_default().push(ix);
        self.rights.entry(snarl.right()).or_default().push(ix);
    }

    pub fn with_boundary(&self, x: Node) -> SnarlMapIter<'_> {
        SnarlMapIter::new(self, x)
    }

    pub fn get_snarl_ix(&self, x: Node, y: Node) -> Option<usize> {
        let left = x.min(y);
        let right = x.max(y);

        let lefts = self.lefts.get(&left)?.iter().collect::<FxHashSet<_>>();
        let rights = self.rights.get(&right)?.iter().collect::<FxHashSet<_>>();

        let mut intersection = lefts.intersection(&rights);

        let snarl_ix = intersection.next()?;

        Some(**snarl_ix)
    }

    pub fn get(&self, x: Node, y: Node) -> Option<Snarl<()>> {
        let left = x.min(y);
        let right = x.max(y);

        let lefts = self.lefts.get(&left)?.iter().collect::<FxHashSet<_>>();
        let rights = self.rights.get(&right)?.iter().collect::<FxHashSet<_>>();

        let mut intersection = lefts.intersection(&rights);

        let snarl_ix = intersection.next()?;

        let snarl = self.snarls.get(snarl_ix)?;

        Some(*snarl)
    }

    pub fn mark_snarl(
        &mut self,
        x: Node,
        y: Node,
        bridge: Node,
        contains: bool,
    ) -> Option<()> {
        let snarl_ix = self.get_snarl_ix(x, y)?;

        let snarl_contains = self.snarl_contains.entry(snarl_ix).or_default();

        let bridge_canonical = bridge.left();

        snarl_contains.insert(bridge_canonical, contains);

        Some(())
    }

    pub fn snarl_contains(
        &self,
        x: Node,
        y: Node,
    ) -> Option<&FxHashMap<Node, bool>> {
        let snarl_ix = self.get_snarl_ix(x, y)?;

        self.snarl_contains.get(&snarl_ix)
    }

    /// Returns a map from black bridge edges to snarls containing the edge
    pub fn invert_contains(&self) -> FxHashMap<Node, FxHashSet<Snarl<()>>> {
        let mut res: FxHashMap<Node, FxHashSet<Snarl<()>>> = Default::default();

        for (&snarl_ix, contained) in self.snarl_contains.iter() {
            let snarl = *self.snarls.get(&snarl_ix).unwrap();

            for (&bridge, &contains) in contained.iter() {
                if contains {
                    res.entry(bridge).or_default().insert(snarl);
                }
            }
        }

        res
    }
}