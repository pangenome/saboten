use log::{debug, trace};

use rustc_hash::{FxHashMap, FxHashSet};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Biedged {}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cactus {}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bridge {}

pub trait GraphType {}

impl GraphType for Biedged {}
impl GraphType for Cactus {}
impl GraphType for Bridge {}

/// A node index for a biedged graph of the specified type
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Node<G> {
    _graph: std::marker::PhantomData<G>,
    id: u64,
}

impl<G> Node<G> {
    #[inline]
    pub fn new_for<H>(id: u64) -> Node<H> {
        Node {
            _graph: std::marker::PhantomData,
            id,
        }
    }

    #[inline]
    pub fn new(id: u64) -> Self {
        Node {
            _graph: std::marker::PhantomData,
            id,
        }
    }

    #[inline]
    pub fn map_graph_type<H>(&self) -> Node<H> {
        Node {
            _graph: std::marker::PhantomData,
            id: self.id,
        }
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

    /// Return the opposite node
    #[inline]
    pub fn opposite(&self) -> Self {
        Self {
            id: self.id ^ 1,
            ..*self
        }
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
    left: Node<Biedged>,
    right: Node<Biedged>,
    ty: SnarlType,
    data: T,
}

impl<T> Snarl<T>
where
    T: Default + Copy + Eq + Ord + std::hash::Hash,
{
    pub fn chain_pair(x: Node<Biedged>, y: Node<Biedged>) -> Self {
        let left = x.min(y);
        let right = x.max(y);

        Snarl {
            left,
            right,
            ty: SnarlType::ChainPair,
            data: T::default(),
        }
    }

    pub fn bridge_pair(x: Node<Biedged>, y: Node<Biedged>) -> Self {
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
    pub fn left(&self) -> Node<Biedged> {
        self.left
    }

    pub fn right(&self) -> Node<Biedged> {
        self.left
    }

    pub fn snarl_type(&self) -> SnarlType {
        self.ty
    }

    pub fn data(&self) -> T {
        self.data
    }

    pub fn chain_pair_with(
        x: Node<Biedged>,
        y: Node<Biedged>,
        data: T,
    ) -> Self {
        let left = x.min(y);
        let right = x.max(y);

        Snarl {
            left,
            right,
            ty: SnarlType::ChainPair,
            data,
        }
    }

    pub fn bridge_pair_with(
        x: Node<Biedged>,
        y: Node<Biedged>,
        data: T,
    ) -> Self {
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

pub struct SnarlMap {
    // Snarls indexed by left boundary
    lefts: FxHashMap<Node<Biedged>, Vec<usize>>,
    // Snarls indexed by right boundary
    rights: FxHashMap<Node<Biedged>, Vec<usize>>,

    // Snarls by rank
    snarls: FxHashMap<usize, Snarl<()>>,

    // Map of contained/not contained black edges for each snarl by rank
    snarl_contains: FxHashMap<usize, FxHashMap<Node<Biedged>, bool>>,
}

pub struct SnarlMapIter<'a> {
    lefts: Option<std::slice::Iter<'a, usize>>,
    rights: Option<std::slice::Iter<'a, usize>>,

    snarls: &'a FxHashMap<usize, Snarl<()>>,
}

impl<'a> SnarlMapIter<'a> {
    fn new(snarl_map: &'a SnarlMap, x: Node<Biedged>) -> Self {
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
    pub fn with_boundary(&self, x: Node<Biedged>) -> SnarlMapIter<'_> {
        SnarlMapIter::new(self, x)
    }

    pub fn get_snarl_ix(
        &self,
        x: Node<Biedged>,
        y: Node<Biedged>,
    ) -> Option<usize> {
        let left = x.min(y);
        let right = x.max(y);

        let lefts = self.lefts.get(&left)?.iter().collect::<FxHashSet<_>>();
        let rights = self.rights.get(&right)?.iter().collect::<FxHashSet<_>>();

        let mut intersection = lefts.intersection(&rights);

        let snarl_ix = intersection.next()?;

        Some(**snarl_ix)
    }

    pub fn get(&self, x: Node<Biedged>, y: Node<Biedged>) -> Option<Snarl<()>> {
        let left = x.min(y);
        let right = x.max(y);

        let lefts = self.lefts.get(&left)?.iter().collect::<FxHashSet<_>>();
        let rights = self.rights.get(&right)?.iter().collect::<FxHashSet<_>>();

        let mut intersection = lefts.intersection(&rights);

        let snarl_ix = intersection.next()?;

        let snarl = self.snarls.get(snarl_ix)?;

        Some(*snarl)
    }
}
