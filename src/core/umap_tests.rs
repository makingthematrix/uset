#[cfg(test)]
mod umap_tests {
    use crate::core::umap::*;
    use crate::core::uset::*;
    use spectral::prelude::*;

    #[test]
    fn should_do_basic_operations() {
        let mut map = UMap::new() as UMap<bool>;
        assert_that!(map.is_empty()).is_true();
        map.put(5, true);
        assert_that!(map.is_empty()).is_false();
        assert_that!(map.len()).is_equal_to(1);
        assert_that!(map.contains(5)).is_true();
        assert_that!(map.contains(4)).is_false();
        assert_eq!(Some(5), map.min());
        assert_eq!(Some(5), map.max());
        map.put(2, false);
        assert_that!(map.len()).is_equal_to(2);
        assert_eq!(Some(2), map.min());
        assert_eq!(Some(5), map.max());
        assert_that!(map.get(2)).is_equal_to(Some(false));
        let re1 = map.remove(5);
        assert_that!(re1).is_equal_to(Some(true));
        assert_that!(map.len()).is_equal_to(1);
        let re2 = map.remove(1);
        assert_that!(re2).is_equal_to(None);
        map.remove(2);
        assert_that!(map.is_empty()).is_true();

        assert_that!(map.get(12)).is_equal_to(None);
    }

    #[test]
    fn should_impl_basic_iterator() {
        let vec = vec![None, None, Some(2), None, Some(4), Some(5)];
        let mut iter = vec.iter().enumerate().filter_map(|(i, &v)| {
            if v.is_some() {
                Some((i, v.unwrap()))
            } else {
                None
            }
        });
        assert_that!(iter.next()).is_equal_to(Some((2, 2)));
        assert_that!(iter.next()).is_equal_to(Some((4, 4)));
        assert_that!(iter.next()).is_equal_to(Some((5, 5)));
        assert_that!(iter.next()).is_equal_to(None);
    }

    #[test]
    fn should_impl_better_iterator() {
        let mut map = UMap::new();
        map.put(2, 2);
        map.put(4, 4);
        map.put(5, 5);
        /* TODO: I need an awesome macro that would let me write it like:
            ```
            let map = umap!(2 -> 2, 4 -> 4, 5 -> 5)
            ```
        */

        let mut iter = map.iter();

        assert_that!(iter.next()).is_equal_to(Some((2, &2)));
        assert_that!(iter.next()).is_equal_to(Some((4, &4)));
        assert_that!(iter.next()).is_equal_to(Some((5, &5)));
        assert_that!(iter.next()).is_equal_to(None);
    }

    #[test]
    fn should_min_max() {
        let map: UMap<&str> = vec![(2, "a"), (4, "b"), (5, "c")].into();

        assert_that!(map.min()).is_equal_to(Some(2));
        assert_that!(map.max()).is_equal_to(Some(5));
    }

    #[test]
    fn should_min_max_when_empty() {
        let map: UMap<&str> = UMap::new();

        assert_that!(map.min()).is_equal_to(None);
        assert_that!(map.max()).is_equal_to(None);
    }

    #[test]
    fn should_join_maps() {
        let map1: UMap<i32> = vec![(2, 2), (4, 4), (5, 5)].into();
        let mut iter1 = map1.iter();
        assert_that!(iter1.next()).is_equal_to(Some((2, &2)));
        assert_that!(iter1.next()).is_equal_to(Some((4, &4)));
        assert_that!(iter1.next()).is_equal_to(Some((5, &5)));
        assert_that!(iter1.next()).is_equal_to(None);

        let map2: UMap<i32> = vec![(1, 1), (3, 3), (5, 5), (8, 8)].into();
        let mut iter2 = map2.iter();
        assert_that!(iter2.next()).is_equal_to(Some((1, &1)));
        assert_that!(iter2.next()).is_equal_to(Some((3, &3)));
        assert_that!(iter2.next()).is_equal_to(Some((5, &5)));
        assert_that!(iter2.next()).is_equal_to(Some((8, &8)));
        assert_that!(iter2.next()).is_equal_to(None);

        let map3 = &map1 + &map2;
        assert_that!(map3.len()).is_equal_to(6);
        let mut iter3 = map3.iter();
        assert_that!(iter3.next()).is_equal_to(Some((1, &1)));
        assert_that!(iter3.next()).is_equal_to(Some((2, &2)));
        assert_that!(iter3.next()).is_equal_to(Some((3, &3)));
        assert_that!(iter3.next()).is_equal_to(Some((4, &4)));
        assert_that!(iter3.next()).is_equal_to(Some((5, &5)));
        assert_that!(iter3.next()).is_equal_to(Some((8, &8)));
        assert_that!(iter3.next()).is_equal_to(None);
    }

    #[test]
    fn should_extract_submap() {
        let map1: UMap<i32> = vec![(1, 1), (3, 3), (5, 5), (8, 8)].into();
        let set = uset![3, 5];
        let map2 = map1.submap(&set);
        assert_eq!(2, map2.len());
        assert_that!(map2.get(3)).is_equal_to(Some(3));
        assert_that!(map2.get(5)).is_equal_to(Some(5));

        let res = map1.retrieve(&set);
        assert_eq!(2, res.len());
        assert_that!(res[0]).is_equal_to(3);
        assert_that!(res[1]).is_equal_to(5);
    }

    #[test]
    fn should_use_umap_macro() {
        let map1 = UMap::from_slice(&[(0, "a"), (1, "b"), (2, "c")]);
        let map2 = umap![(0, "a"), (1, "b"), (2, "c")];
        assert_eq!(map1, map2);
    }

    #[test]
    fn should_modify_with_get_ref_mut() {
        let mut map = UMap::from_slice(&[(0, "a"), (1, "b"), (2, "c")]);
        assert_eq!(Some(&"b"), map.get_ref(1));
        if let Some(v) = map.get_ref_mut(1) {
            *v = "d";
        }
        assert_eq!(Some(&"d"), map.get_ref(1));
    }
}
