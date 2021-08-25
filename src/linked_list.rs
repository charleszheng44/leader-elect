pub struct List<T> {
    head: Link<T>,
}

struct Node<T> {
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

impl<T> List<T> {
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
}
