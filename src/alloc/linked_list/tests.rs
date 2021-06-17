#![cfg(test)]

use super::LinkedList;
use crate::alloc::linked_list::node::Node;
use paste::paste;
use std::{collections::LinkedList as StdLibList, fmt::Debug, ptr::null_mut};

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

fn test_linear_inserts<T, const N: usize>()
where
    T: Sized + Copy + Eq + Debug + From<u8> + 'static,
{
    let src: Vec<u8> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];

    let mut std_ll = StdLibList::new();
    let mut ll = LinkedList::<T, N>::new();
    let mut c = ll.cursor_mut();
    for i in src {
        let i: T = i.into();

        c.next();
        c.insert_after(i);
        validate(&mut c.list);

        std_ll.push_back(i);
        compare_lists(&std_ll, &mut c.list);
    }
}

fn test_collect<T, const N: usize>()
where
    T: Sized + Clone + Eq + Debug + From<u8> + 'static,
{
    let src: Vec<u8> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
    macro_rules! copy {
        () => {
            src.iter().cloned().map(|v| v.into()).collect()
        };
    }
    let std: StdLibList<T> = copy!();
    let mut ll: LinkedList<T, N> = copy!();
    validate(&mut ll);
    compare_lists(&std, &mut ll);
}

// TODO: seeking tests
// TODO: various removal tests
// TODO: fuzzing test with no references
// TODO: fuzzing test with references
// TODO: 100% coverage

// Generate tests with various node sizes and integer types
macro_rules! make_tests {
    ($( $type:ty => $size:literal )*) => {
        $(
            paste! {
                #[test]
                fn [<linear_inserts_ $type _ $size>]() {
                    test_linear_inserts::<$type, $size>();
                }

                #[test]
                fn [<collect_ $type _ $size>]() {
                    test_collect::<$type, $size>();
                }
            }
        )*
    };
    ($( $size:literal )*) => {
        $(
            make_tests!{
                u8 => $size
                u64 => $size
                u128 => $size
            }
        )*
    }
}

make_tests! {4 1 2 8 15 16 17 32 64 128}
