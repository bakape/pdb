#![cfg(test)]

use super::LinkedList;
use crate::alloc::linked_list::node::Node;
use std::{collections::LinkedList as StdLibList, fmt::Debug, ptr::null_mut};

// Generate tests with various node sizes
macro_rules! gen_tests {
    ($name:ident) => {
        mod $name {
            // Start with 4, so the first `Run Test` in the IDE is
            // has multiple values per node
            gen_tests! {@for_sizes $name {4 1 2 8 15 16 17 32 64 128}}
        }
    };
    (@for_sizes $name:ident { $( $size:literal )* }) => {
        $(
            paste::paste! {
                #[test]
                fn [<size_ $size>]() {
                    super::$name::<$size>();
                }
            }
        )*
    };
}

gen_tests! {test_linear_inserts}
fn test_linear_inserts<const N: usize>() {
    let src = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];

    let mut std_ll = StdLibList::new();
    let mut ll = LinkedList::<usize, N>::new();
    let mut c = ll.cursor_mut();
    for i in src {
        c.next();
        c.insert_after(i);
        validate(&mut c.list);

        std_ll.push_back(i);
        compare_lists(&std_ll, &mut c.list);
    }
}

gen_tests! {test_collect}
fn test_collect<const N: usize>() {
    let src = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
    macro_rules! copy {
        () => {
            src.iter().cloned().collect()
        };
    }
    let std: StdLibList<usize> = copy!();
    let mut ll: LinkedList<usize, N> = copy!();
    validate(&mut ll);
    compare_lists(&std, &mut ll);
}

// TODO: seeking tests
// TODO: various removal tests
// TODO: fuzzing test with no references
// TODO: fuzzing test with references
// TODO: 100% coverage

/// Validate the various components of the list are consistent with each other
fn validate<T, const N: usize>(ll: &mut LinkedList<T, N>)
where
    T: Sized + Clone + Eq + Debug + 'static,
{
    let mut iterations = 0;
    let mut c = ll.cursor_mut();
    if c.value().is_some() {
        iterations = 1;
    }
    while c.next() {
        iterations += 1;
    }
    assert_eq!(iterations, ll.len());

    assert_ne!(ll.head, null_mut());
    assert_ne!(ll.tail, null_mut());
    if ll.len() == 0 {
        assert_eq!(ll.tail, ll.head);
    }

    let mut node_length = 0;
    let mut node = ll.head;
    let mut prev: *mut Node<T, N> = null_mut();
    while node != null_mut() {
        unsafe {
            node_length += (*node).len();

            if prev != null_mut() {
                assert_eq!((*prev).next(), node);
            }
            assert_eq!((*node).previous(), prev);
            prev = node;

            node = (*node).next();
        }
    }
    assert_eq!(node_length, ll.len());
}

/// Assert the list from standard library and allocator list are equal.
// Also perform basic consistency validation.
fn compare_lists<T, const N: usize>(
    std: &StdLibList<T>,
    ll: &mut LinkedList<T, N>,
) where
    T: Sized + Clone + Eq + Debug + 'static,
{
    macro_rules! compare_it {
        ($expected:expr, $got:expr) => {
            assert_eq!(
                $expected.cloned().collect::<Vec<_>>(),
                $got.map(|v| v.clone()).collect::<Vec<_>>(),
            );
        };
    }

    compare_it!(std.iter(), ll.iter_mut());
    compare_it!(std.iter().rev(), ll.iter_mut_reverse());
}
