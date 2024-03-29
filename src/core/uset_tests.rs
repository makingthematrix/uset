#[cfg(test)]
mod uset_tests {
    use crate::core::uset::*;

    use std::collections::HashSet;

    use quickcheck::TestResult;
    use spectral::prelude::*;

    #[test]
    fn set_from_and_to_vec() {
        let v = vec![0, 3, 8, 10];
        let s: USet = USet::from(&v);

        assert_that(&(s.len()))
            .named(&"USet length")
            .is_equal_to(&4);
        assert_that(&(s.capacity()))
            .named(&"USet capacity")
            .is_equal_to(&11);

        assert_that(&(s.contains(0))).is_true();
        assert_that(&(s.contains(3))).is_true();
        assert_that(&(s.contains(8))).is_true();
        assert_that(&(s.contains(10))).is_true();
        assert_that(&(s.contains(9))).is_false();

        let v2: Vec<usize> = s.into();

        assert_that!(&v2).is_equal_to(&v);
    }

    fn to_unique_sorted_vec(v: &Vec<usize>) -> Vec<usize> {
        let mut hs = HashSet::new();
        for x in v {
            hs.insert(*x);
        }

        let mut v2: Vec<usize> = hs.into_iter().collect();
        v2.sort();
        v2
    }

    fn vec_compare(va: &[usize], vb: &[usize]) -> bool {
        (va.len() == vb.len()) &&  // zip stops at the shortest
            va.iter()
                .zip(vb)
                .all(|(&a, &b)| a == b)
    }

    quickcheck! {
        fn from_and_to_vec(v: Vec<usize>) -> TestResult {
            let unique_v = to_unique_sorted_vec(&v);

            if v.len() != unique_v.len() {
                return TestResult::discard()
            }

            let result: Vec<usize> = USet::from(&unique_v).into_iter().collect();
            TestResult::from_bool(vec_compare(&unique_v, &result))
        }
    }

    #[test]
    fn should_substract() {
        let s1 = uset![0, 3, 8, 10];
        let s2 = uset![3, 8];
        let s5 = USet::new();

        let s3 = &s1 - &s2;

        assert_that(&(s3.len())).is_equal_to(&2);
        assert_that(&(s3.contains(0))).is_true();
        assert_that(&(s3.contains(10))).is_true();

        let s4 = &s1 - &s2;

        assert_that(&(s4.len())).is_equal_to(&2);
        assert_that(&(s4.contains(0))).is_true();
        assert_that(&(s4.contains(10))).is_true();

        assert_that!((&s1 - &s5)).is_equal_to(s1.clone());
        assert_that!((&s5 - &s5)).is_equal_to(USet::new());
    }

    #[test]
    fn should_compile() {
        let s4 = vec![0usize, 3, 8, 10];
        for _i in 1..10 {
            let _s5: USet = USet::from(&s4);
        }
    }

    #[test]
    fn should_be_equal() {
        let s1 = uset![0, 3, 8, 10];
        let s2 = uset![0, 3, 8, 10];
        assert_that(&s1).is_equal_to(&s2);
        assert_that(&(s1 == s2)).is_true();
    }

    #[test]
    fn should_find_min() {
        let s1 = uset![0, 3, 8, 10];
        assert_that(&(s1.iter().next())).is_equal_to(&Some(0));
        let s2 = uset![3, 8, 10];
        assert_that(&(s2.iter().next())).is_equal_to(&Some(3));
        let s3 = USet::new();
        let mut s3iter = s3.iter();
        assert_that(&(s3iter.next())).is_equal_to(&None);
        assert_that(&(s3iter.next())).is_equal_to(&None);

        let mut s2iter = s2.iter();
        assert_that!(s2iter.next()).is_equal_to(Some(3));
        assert_that!(s2iter.next()).is_equal_to(Some(8));
        assert_that!(s2iter.next()).is_equal_to(Some(10));
        assert_that!(s2iter.next()).is_equal_to(None);
        assert_that!(s2iter.next()).is_equal_to(None);

        let s4 = uset![0];
        let mut s4iter = s4.iter();
        assert_that!(s4iter.next()).is_equal_to(Some(0));
        assert_that!(s4iter.next()).is_equal_to(None);
        assert_that!(s4iter.next()).is_equal_to(None);

        // TODO: find min after adding a new element, smaller than the previous min

        // TODO: find min after removing the previous min
    }

    #[test]
    fn should_find_max() {
        let s1 = uset![0, 3, 8, 10];
        assert_that!(s1.iter().rev().next()).is_equal_to(Some(10));
        let s2 = uset![0];
        assert_that!(s2.iter().rev().next()).is_equal_to(Some(0));
        let s3 = USet::new();
        let mut s3iter = s3.iter().rev();
        assert_that!(s3iter.next()).is_equal_to(None);
        assert_that!(s3iter.next()).is_equal_to(None);

        let mut s2iter = s2.iter().rev();
        assert_that!(s2iter.next()).is_equal_to(Some(0));
        assert_that!(s2iter.next()).is_equal_to(None);
        assert_that!(s2iter.next()).is_equal_to(None);

        // TODO: find max after adding a new element, bigger than the previous max

        // TODO: find max after removing the previous max
    }

    #[test]
    fn should_add() {
        let s1 = uset![0, 3, 8, 10];
        let s2 = uset![1, 4];
        let s3 = uset![3, 5];
        let s4 = USet::new();

        assert_that!((&s1 + &s2)).is_equal_to(uset![0, 1, 3, 4, 8, 10]);
        assert_that!((&s1 + &s3)).is_equal_to(uset![0, 3, 5, 8, 10]);
        assert_that!((&s1 + &s4)).is_equal_to(s1.clone());
        assert_that!((&s1 + &s1)).is_equal_to(s1.clone());
        assert_that!((&s4 + &s4)).is_equal_to(s4.clone());
    }

    #[test]
    fn should_push_all() {
        let mut s1 = uset![0, 3, 8, 10];
        s1.push_all(&vec![1, 4]);
        assert_that!(&s1).is_equal_to(uset![0, 1, 3, 4, 8, 10]);

        let mut s2 = uset![0, 3, 8, 10];
        s2.push_all(&Vec::<usize>::new());
        assert_that!(&s2).is_equal_to(uset![0, 3, 8, 10]);

        let mut s3 = uset![3, 8, 10];
        s3.push_all(&vec![1, 4]);
        assert_that!(&s3).is_equal_to(uset![1, 3, 4, 8, 10]);

        let mut s4 = uset![3, 8, 10];
        s4.push_all(&vec![6, 12]);
        assert_that!(&s4).is_equal_to(uset![3, 6, 8, 10, 12]);

        let mut s5 = uset![3, 8, 10];
        s5.push_all(&vec![1, 14]);
        assert_that!(&s5).is_equal_to(uset![1, 3, 8, 10, 14]);

        let mut s6 = uset![3, 8, 10];
        s6.push_all(&vec![8, 10, 12]);
        assert_that!(&s6).is_equal_to(uset![3, 8, 10, 12]);
    }

    #[test]
    fn should_join_sets() {
        let set1 = uset![2, 4, 5];
        let mut iter1 = set1.iter();
        assert_that!(iter1.next()).is_equal_to(Some(2));
        assert_that!(iter1.next()).is_equal_to(Some(4));
        assert_that!(iter1.next()).is_equal_to(Some(5));
        assert_that!(iter1.next()).is_equal_to(None);

        let set2 = uset![1, 3, 5, 8];
        let mut iter2 = set2.iter();
        assert_that!(iter2.next()).is_equal_to(Some(1));
        assert_that!(iter2.next()).is_equal_to(Some(3));
        assert_that!(iter2.next()).is_equal_to(Some(5));
        assert_that!(iter2.next()).is_equal_to(Some(8));
        assert_that!(iter2.next()).is_equal_to(None);

        let set3 = &set1 + &set2;
        assert_that!(set3.len()).is_equal_to(6);
        let mut iter3 = set3.iter();
        assert_that!(iter3.next()).is_equal_to(Some(1));
        assert_that!(iter3.next()).is_equal_to(Some(2));
        assert_that!(iter3.next()).is_equal_to(Some(3));
        assert_that!(iter3.next()).is_equal_to(Some(4));
        assert_that!(iter3.next()).is_equal_to(Some(5));
        assert_that!(iter3.next()).is_equal_to(Some(8));
        assert_that!(iter3.next()).is_equal_to(None);
    }

    #[test]
    fn should_mul() {
        let s1 = uset![0, 3, 8, 10];
        let s2 = uset![3, 8];
        assert_that!((&s1 * &s2)).is_equal_to(uset![3, 8]);

        let s3 = uset![1, 2, 3];
        assert_that!((&s1 * &s3)).is_equal_to(uset![3]);

        let s4 = USet::new();
        assert_that!((&s1 * &s4)).is_equal_to(USet::new());

        assert_that!((&s1 * &s1)).is_equal_to(s1.clone());

        let s5 = uset![2, 4, 6];
        assert_that!((&s1 * &s5)).is_equal_to(USet::new());

        let s6 = uset![10];
        assert_that!((&s1 * &s6)).is_equal_to(s6.clone());
    }

    #[test]
    fn should_xor() {
        let s1 = uset![0, 3, 8, 10];

        let s2 = uset![3, 8];
        assert_that!((&s1 ^ &s2)).is_equal_to(uset![0, 10]);

        let s3 = uset![1, 2, 3];
        assert_that!((&s1 ^ &s3)).is_equal_to(uset![0, 1, 2, 8, 10]);

        let s4 = USet::new();
        assert_that!((&s1 ^ &s4)).is_equal_to(s1.clone());

        assert_that!((&s1 ^ &s1)).is_equal_to(USet::new());

        let s5 = uset![2, 4, 6];
        assert_that!((&s1 ^ &s5)).is_equal_to(uset![0, 2, 3, 4, 6, 8, 10]);

        let s6 = uset![10];
        assert_that!((&s1 ^ &s6)).is_equal_to(uset![0, 3, 8]);
    }

    #[test]
    fn should_implement_into_iter() {
        let s = uset![3, 5, 8];
        let mut sum = 0;
        for i in &s {
            sum += i;
        }
        assert_eq!(sum, 16);
    }

    #[test]
    fn should_shrink_to_fit() {
        let mut s = uset![3, 5, 8];
        assert_eq!(3, s.len());
        assert_eq!(INITIAL_WORKING_CAPACITY, s.capacity());
        s.remove(3);
        assert_eq!(2, s.len());
        assert_eq!(INITIAL_WORKING_CAPACITY, s.capacity());
        s.shrink_to_fit();
        assert_eq!(2, s.len());
        assert_eq!(4, s.capacity());
    }

    #[test]
    fn should_substract_sets() {
        let set1 = uset![2, 4, 5];
        assert_eq!(Some(2), set1.min());
        assert_eq!(Some(5), set1.max());
        let set2 = uset![1, 3, 5, 8];

        let set3 = &set1 - &set2;
        assert_that!(set3.len()).is_equal_to(2);
        let mut iter3 = set3.iter();
        assert_that!(iter3.next()).is_equal_to(Some(2));
        assert_that!(iter3.next()).is_equal_to(Some(4));
        assert_that!(iter3.next()).is_equal_to(None);

        assert_eq!(Some(2), set3.min());
        assert_eq!(Some(4), set3.max());
    }

    #[test]
    fn should_make_set_from_iter() {
        let vec = vec![3usize, 5, 8, 11];
        let set: USet = vec
            .iter()
            .filter_map(|&n| if n % 2 != 0 { Some(n) } else { None })
            .collect();
        assert_that!(set.contains(3));
        assert_that!(set.contains(5));
        assert_that!(set.contains(11));
        assert_that!(set.contains(8) == false);
    }
}
