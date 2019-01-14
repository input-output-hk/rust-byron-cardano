use std::collections::{BTreeMap, BTreeSet};

/// Compute the diff from BTreeMap 'm1' to BTreeMap 'm2', returning
/// the set of keys in 'm1' that are not in 'm2', and the map of
/// keys/values that are in 'm2' but not in 'm1'.
pub fn diff_maps<'a, K, V>(
    m1: &'a BTreeMap<K, V>,
    m2: &'a BTreeMap<K, V>,
) -> (BTreeSet<&'a K>, BTreeMap<&'a K, &'a V>)
where
    K: Ord,
{
    let mut removed = BTreeSet::new();
    let mut added = BTreeMap::new();

    let mut i1 = m1.iter();
    let mut i2 = m2.iter();

    let mut e1 = i1.next();
    let mut e2 = i2.next();

    loop {
        match e1 {
            None => match e2 {
                None => break,
                Some((n2, v2)) => {
                    added.insert(n2, v2);
                    e2 = i2.next();
                }
            },
            Some((n1, _)) => match e2 {
                None => {
                    removed.insert(n1);
                    e1 = i1.next();
                }
                Some((n2, v2)) => {
                    if n1 < n2 {
                        removed.insert(n1);
                        e1 = i1.next();
                    } else if n1 > n2 {
                        added.insert(n2, v2);
                        e2 = i2.next();
                    } else {
                        e1 = i1.next();
                        e2 = i2.next();
                    }
                }
            },
        };
    }

    (removed, added)
}
