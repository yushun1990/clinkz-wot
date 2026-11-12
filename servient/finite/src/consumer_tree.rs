//! Private, individually funded AVL nodes. No resizing, hashing or row aliases.
use super::*;

type Link<V> = Option<Box<Node<V>>>;
pub(super) struct Node<V> {
    pub key: Rc<RegisteredName>,
    pub value: V,
    left: Link<V>,
    right: Link<V>,
    height: u8,
    // Last: payload and both subtrees retire before backing is refunded.
    _funding: HostCapacityReservation,
}
pub(super) struct Tree<V> {
    root: Link<V>,
    len: usize,
}
impl<V> Default for Tree<V> {
    fn default() -> Self {
        Self { root: None, len: 0 }
    }
}
impl<V> Tree<V> {
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn lookup_work(&self, key: &str, budget: &mut WorkBudget) -> CoreResult<()> {
        charge(
            budget,
            height(&self.root) as u64 * (key.len() as u64 + 1) + 1,
        )
    }
    pub fn mutation_work(&self, key: &str, budget: &mut WorkBudget) -> CoreResult<()> {
        // Deletion visits at most two height-bounded spines (search and
        // successor extraction). Each frame does one comparison and at most
        // two rotations; 64 fixed metadata operations cover its rewiring and
        // height updates. Insertion uses only one of those spines.
        charge(
            budget,
            2 * (height(&self.root) as u64 + 1) * (key.len() as u64 + 65),
        )
    }
    pub fn prepare(
        key: Rc<RegisteredName>,
        value: V,
        bank: &Arc<HostResourceAccounts>,
    ) -> Result<Box<Node<V>>, V> {
        let Some(funding) = bank.reserve(
            K::EngineLiveBytesGlobalMax,
            core::mem::size_of::<Node<V>>() as u64,
        ) else {
            return Err(value);
        };
        Ok(Box::new(Node {
            key,
            value,
            left: None,
            right: None,
            height: 1,
            _funding: funding,
        }))
    }
    pub fn get(&self, key: &str) -> Option<&V> {
        let mut next = self.root.as_deref();
        while let Some(node) = next {
            match key.cmp(node.key.value.as_ref()) {
                Ordering::Less => next = node.left.as_deref(),
                Ordering::Greater => next = node.right.as_deref(),
                Ordering::Equal => return Some(&node.value),
            }
        }
        None
    }
    pub fn get_mut(&mut self, key: &str) -> Option<&mut V> {
        let mut next = self.root.as_deref_mut();
        while let Some(node) = next {
            match key.cmp(node.key.value.as_ref()) {
                Ordering::Less => next = node.left.as_deref_mut(),
                Ordering::Greater => next = node.right.as_deref_mut(),
                Ordering::Equal => return Some(&mut node.value),
            }
        }
        None
    }
    // Caller prepays mutation_work and proves absence before allocating. No
    // linked code runs between that check and this nonfallible local commit.
    pub fn insert_absent(&mut self, node: Box<Node<V>>) {
        self.root = Some(insert(self.root.take(), node));
        self.len += 1;
    }
    // Returned node has no children. Its value, key and backing are dropped by
    // the caller after registry borrows end, never by balancing/removal.
    pub fn remove(&mut self, key: &str) -> Option<Box<Node<V>>> {
        let (root, removed) = remove(self.root.take(), key);
        self.root = root;
        self.len -= usize::from(removed.is_some());
        removed
    }
    pub fn visit(&self, mut f: impl FnMut(&V)) {
        fn walk<V>(node: &Link<V>, f: &mut impl FnMut(&V)) {
            if let Some(n) = node {
                walk(&n.left, f);
                f(&n.value);
                walk(&n.right, f);
            }
        }
        walk(&self.root, &mut f);
    }
}
fn height<V>(node: &Link<V>) -> u8 {
    node.as_ref().map_or(0, |n| n.height)
}
fn fix<V>(node: &mut Node<V>) {
    node.height = 1 + height(&node.left).max(height(&node.right));
}
fn left<V>(mut root: Box<Node<V>>) -> Box<Node<V>> {
    let mut next = root.right.take().unwrap();
    root.right = next.left.take();
    fix(&mut root);
    next.left = Some(root);
    fix(&mut next);
    next
}
fn right<V>(mut root: Box<Node<V>>) -> Box<Node<V>> {
    let mut next = root.left.take().unwrap();
    root.left = next.right.take();
    fix(&mut root);
    next.right = Some(root);
    fix(&mut next);
    next
}
fn balance<V>(mut node: Box<Node<V>>) -> Box<Node<V>> {
    fix(&mut node);
    if height(&node.left) > height(&node.right) + 1 {
        let l = node.left.as_ref().unwrap();
        if height(&l.right) > height(&l.left) {
            node.left = Some(left(node.left.take().unwrap()));
        }
        return right(node);
    }
    if height(&node.right) > height(&node.left) + 1 {
        let r = node.right.as_ref().unwrap();
        if height(&r.left) > height(&r.right) {
            node.right = Some(right(node.right.take().unwrap()));
        }
        return left(node);
    }
    node
}
fn insert<V>(root: Link<V>, node: Box<Node<V>>) -> Box<Node<V>> {
    let Some(mut root) = root else {
        return node;
    };
    match node.key.value.cmp(&root.key.value) {
        Ordering::Less => root.left = Some(insert(root.left.take(), node)),
        Ordering::Greater => root.right = Some(insert(root.right.take(), node)),
        Ordering::Equal => unreachable!("prechecked unique key"),
    }
    balance(root)
}
fn take_min<V>(mut root: Box<Node<V>>) -> (Link<V>, Box<Node<V>>) {
    let Some(child) = root.left.take() else {
        return (root.right.take(), root);
    };
    let (child, min) = take_min(child);
    root.left = child;
    (Some(balance(root)), min)
}
fn remove<V>(root: Link<V>, key: &str) -> (Link<V>, Link<V>) {
    let Some(mut root) = root else {
        return (None, None);
    };
    let removed = match key.cmp(root.key.value.as_ref()) {
        Ordering::Less => {
            let (child, removed) = remove(root.left.take(), key);
            root.left = child;
            removed
        }
        Ordering::Greater => {
            let (child, removed) = remove(root.right.take(), key);
            root.right = child;
            removed
        }
        Ordering::Equal => {
            let Some(r) = root.right.take() else {
                return (root.left.take(), Some(root));
            };
            let (r, mut successor) = take_min(r);
            successor.left = root.left.take();
            successor.right = r;
            return (Some(balance(successor)), Some(root));
        }
    };
    (Some(balance(root)), removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::collections::BTreeMap;
    use clinkz_wot_foundation::{GatewayDefaultV1, StaticResourceProfile};
    fn inspect<V>(node: &Link<V>, low: Option<&str>, high: Option<&str>) -> (u8, usize) {
        let Some(n) = node else {
            return (0, 0);
        };
        let k = n.key.value.as_ref();
        assert!(low.is_none_or(|l| l < k) && high.is_none_or(|h| k < h));
        let (l, lc) = inspect(&n.left, low, Some(k));
        let (r, rc) = inspect(&n.right, Some(k), high);
        assert!(l.abs_diff(r) <= 1);
        assert_eq!(n.height, 1 + l.max(r));
        (n.height, lc + rc + 1)
    }
    #[test]
    fn mixed_tree_mutations_preserve_order_balance_count_and_original_values() {
        let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
        let mut tree = Tree::default();
        let mut model = BTreeMap::new();
        let mut seed = 7u64;
        for turn in 0..20_000 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let key = alloc::format!("common-prefix-{:04}", (seed >> 32) % 512);
            if seed & 3 == 0 {
                assert_eq!(tree.remove(&key).map(|n| n.value), model.remove(&key));
            } else if !model.contains_key(&key) {
                let name = Rc::new(RegisteredName {
                    value: key.clone().into_boxed_str(),
                    _funding: bank
                        .reserve(K::EngineLiveBytesGlobalMax, key.len() as u64 + 128)
                        .unwrap(),
                });
                tree.insert_absent(Tree::prepare(name, turn, &bank).ok().unwrap());
                model.insert(key, turn);
            } else {
                assert_eq!(tree.get(&key), model.get(&key));
            }
            assert_eq!(inspect(&tree.root, None, None).1, tree.len());
            assert_eq!(tree.len(), model.len());
        }
        for (key, value) in model {
            assert_eq!(tree.remove(&key).unwrap().value, value);
        }
        drop(tree);
        assert_eq!(bank.account(K::EngineLiveBytesGlobalMax).used(), 0);
    }
}
