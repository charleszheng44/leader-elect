use serde::{Deserialize, Serialize};
use std::cmp::PartialEq;

#[derive(Debug, Serialize, Deserialize)]
pub struct List<T: PartialEq> {
    head: Link<T>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Node<T: PartialEq> {
    ele: T,
    next: Link<T>,
}

type Link<T> = Option<Box<Node<T>>>;

macro_rules! new_box_node {
    ($ele:expr, $next:expr) => {
        Box::new(Node {
            ele: $ele,
            next: $next,
        })
    };
}

impl<'a, T: PartialEq> Iterator for Iter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        self.next.take().map(|node| {
            self.next = node.next.as_deref();
            &node.ele
        })
    }
}

pub struct Iter<'a, T: PartialEq> {
    next: Option<&'a Node<T>>,
}

pub struct IterMut<'a, T: PartialEq> {
    next: Option<&'a mut Node<T>>,
}

impl<'a, T: PartialEq> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;
    fn next(&mut self) -> Option<Self::Item> {
        self.next.take().map(|node| {
            self.next = node.next.as_deref_mut();
            &mut node.ele
        })
    }
}

impl<T: PartialEq> List<T> {
    pub fn new() -> List<T> {
        List { head: None }
    }

    pub fn push_head(&mut self, ele: T) {
        self.head = Some(new_box_node!(ele, self.head.take()));
    }

    pub fn push_tail(&mut self, ele: T) {
        let mut cur = &mut self.head;
        while let Some(box_node) = cur {
            cur = &mut box_node.next;
        }
        *cur = Some(new_box_node!(ele, None));
    }

    pub fn pop_head(&mut self) -> Option<T> {
        self.head.take().map(|old_head| {
            self.head = old_head.next;
            old_head.ele
        })
    }

    pub fn pop_tail(&mut self) -> Option<T> {
        let mut cur = &mut self.head;
        loop {
            match cur {
                None => return None,
                Some(box_node) if box_node.next.is_none() => {
                    return cur.take().map(|box_node| box_node.ele);
                }
                Some(box_node) => cur = &mut box_node.next,
            }
        }
    }

    pub fn iter(&self) -> Iter<T> {
        Iter {
            next: self.head.as_deref(),
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<T> {
        IterMut {
            next: self.head.as_deref_mut(),
        }
    }
}

impl<T: PartialEq> PartialEq for List<T> {
    fn eq(&self, other: &Self) -> bool {
        !self.ne(other)
    }

    fn ne(&self, other: &Self) -> bool {
        self.iter().zip(other.iter()).any(|(x, y)| x != y)
    }
}

#[cfg(test)]
mod tests {
    use super::List;
    #[test]
    fn serialization() {
        let mut l1 = List::new();
        l1.push_tail(1);
        l1.push_tail(2);
        l1.push_tail(3);
        let jl = serde_json::to_string(&l1).unwrap();
        println!("json format {}", jl);
        let l2: List<i32> = serde_json::from_str(&jl).unwrap();
        assert_eq!(l1, l2);
    }
}
