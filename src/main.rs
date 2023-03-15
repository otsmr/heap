/*
 * Copyright (c) 2023, Tobias Müller <git@tsmr.eu>
 *
 */

use core::fmt::Display;
use core::ops::Index;
use core::ops::IndexMut;

const DEFAULT_CAPACITY: usize = 100;
const HEAP_SIZE: usize = 1_000_000; // 1 MB

static mut THE_HEAP: [u8; HEAP_SIZE] = [0u8; HEAP_SIZE];
static mut CAPACITY_HOLDER: [Option<LinkedListItem>; 100] = [None; 100];
static mut HEAP_USED: LinkedList = LinkedList { start: None };
static mut BLOCK: bool = false;

struct Capacity();
impl Capacity {
    fn as_mut(index: usize) -> &'static mut LinkedListItem {
        unsafe { CAPACITY_HOLDER[index].as_mut().unwrap() }
    }
    fn as_ref(index: usize) -> &'static LinkedListItem {
        unsafe { CAPACITY_HOLDER[index].as_ref().unwrap() }
    }
    fn free(id: usize) {
        unsafe {
            if CAPACITY_HOLDER[id].is_none() {
                return;
            }
            let prev_id = CAPACITY_HOLDER[id].unwrap().prev_id;
            let next_id = CAPACITY_HOLDER[id].unwrap().next_id;
            if let Some(prev_id) = prev_id {
                Capacity::as_mut(prev_id).next_id = next_id;
            }
            CAPACITY_HOLDER[id] = None;
        }
        // todo!("out of heap space");
    }

    fn add(content: VecAllocatedItem, next_id: Option<usize>, prev_id: Option<usize>) -> usize {
        unsafe {
            for (id, holder) in CAPACITY_HOLDER.iter().enumerate() {
                if holder.is_none() {
                    CAPACITY_HOLDER[id] = Some(LinkedListItem {
                        content,
                        prev_id,
                        next_id,
                        id,
                    });
                    return id;
                }
            }
        }
        todo!("out of heap space");
    }
}

#[derive(Clone, Debug, Copy)]
struct LinkedListItem {
    content: VecAllocatedItem,
    prev_id: Option<usize>,
    next_id: Option<usize>,
    id: usize,
}

impl LinkedListItem {
    pub fn next(&self) -> Option<&mut LinkedListItem> {
        Some(Capacity::as_mut(self.next_id?))
    }
}

#[derive(Clone)]
struct LinkedList {
    start: Option<usize>,
}

impl LinkedList {
    fn find_space(&self, size: usize) -> VecAllocatedItem {
        if self.start.is_none() {
            return VecAllocatedItem {
                start: 0,
                end: size,
                capacity_id: None,
            };
        }
        let mut cursor = Capacity::as_ref(self.start.unwrap());
        while cursor.next().is_some() {
            if (cursor.next().unwrap().content.start - cursor.content.end) > size {
                break; // Found free space
            }
            cursor = cursor.next().unwrap();
        }

        if cursor.content.end + size > HEAP_SIZE {
            panic!("out of heap space");
        }

        VecAllocatedItem {
            start: cursor.content.end,
            end: cursor.content.end + size,
            capacity_id: None,
        }
    }

    fn allocate(&'static mut self, size: usize) -> VecAllocatedItem {
        unsafe {
            while BLOCK {}
            BLOCK = true;
        }
        let mut item = self.find_space(size);
        item.capacity_id = Some(self.add(item));
        unsafe {
            BLOCK = false;
        }
        item
    }

    fn add(&'static mut self, content: VecAllocatedItem) -> usize {
        if self.start.is_none() {
            self.start = Some(Capacity::add(content, None, None));
            return self.start.unwrap();
        }
        let mut cursor = Capacity::as_mut(self.start.unwrap());
        while cursor.next().is_some() {
            if cursor.next().unwrap().content.start > content.start {
                let next_id = cursor.next().unwrap().id;
                let id = Capacity::add(content, Some(next_id), Some(cursor.id));
                cursor.next_id = Some(id);
                return id;
            }
            cursor = cursor.next().unwrap();
        }
        let id = Capacity::add(content, None, Some(cursor.id));
        cursor.next_id = Some(id);
        id
    }
}

#[derive(Clone, Copy, Debug)]
struct VecAllocatedItem {
    start: usize,
    end: usize,
    capacity_id: Option<usize>,
}

pub struct Vec {
    capacity: VecAllocatedItem,
    len: usize,
}

impl Vec {
    pub fn with_capacity(capacity: usize) -> Self {
        unsafe {
            let capacity = HEAP_USED.allocate(capacity);
            println!("capacity={:?}", capacity);
            Vec { capacity, len: 0 }
        }
    }
    pub fn new() -> Vec {
        unsafe {
            let capacity = HEAP_USED.allocate(DEFAULT_CAPACITY);
            println!("capacity={:?}", capacity);
            Vec { capacity, len: 0 }
        }
    }
    pub fn push(&mut self, item: u8) {
        if self.len + self.capacity.start + 1 > self.capacity.end {
            todo!("Allocate bigger space");
        }
        unsafe {
            THE_HEAP[self.capacity.start + self.len] = item;
        }
        self.len += 1;
    }
    pub fn pop(&mut self) -> Option<u8> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        unsafe { Some(THE_HEAP[self.capacity.start + self.len + 1]) }
    }
    pub fn as_slice(&self) -> &'static [u8] {
        unsafe { &THE_HEAP[self.capacity.start..self.capacity.start + self.len] }
    }
    pub fn extend_from_slice(&mut self, slice: &[u8]) {
        if self.len + self.capacity.start + slice.len() > self.capacity.end {
            todo!("Allocate bigger space: {} minimum", slice.len());
        }
        for b in slice {
            self.push(*b);
        }
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn len(&self) -> usize {
        self.len
    }
}

impl Display for Vec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            write!(
                f,
                "{:?}",
                &THE_HEAP[self.capacity.start..self.capacity.start + self.len]
            )
        }
    }
}
impl Drop for Vec {
    fn drop(&mut self) {
        println!("free: {:?}", self.capacity.capacity_id);
        unsafe {
            while BLOCK {}
            BLOCK = true;
        }
        Capacity::free(self.capacity.capacity_id.unwrap());
        unsafe {
            BLOCK = false;
        }
    }
}

impl IndexMut<usize> for Vec {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index >= self.len {
            panic!("Index '{index}' of range vec has size of {}", self.len);
        }
        unsafe { &mut THE_HEAP[self.capacity.start + index] }
    }
}
impl Index<usize> for Vec {
    type Output = u8;
    fn index(&self, index: usize) -> &Self::Output {
        if index >= self.len {
            panic!("Index '{index}' of range vec has size of {}", self.len);
        }
        unsafe { &THE_HEAP[self.capacity.start + index] }
    }
}

impl Default for Vec {
    fn default() -> Self {
        Self::new()
    }
}

fn main() {

    let mut test = Vec::new();
    test.push(40);
    println!("Vec={}", test);

}

#[cfg(test)]
mod tests {

    use super::Vec;

    #[test]
    fn vec_push_pop() {
        let mut a = Vec::with_capacity(10);
        a.extend_from_slice(&[102, 103, 104]);
        a[0] = 100;
        {
            let mut a = Vec::with_capacity(30);
            a.extend_from_slice(&[41, 42, 43]);
            a[0] = 55;
            assert_eq!(a.as_slice(), &[55, 42, 43])
        }
        assert_eq!(a.as_slice(), &[100, 103, 104]);

        let mut a = Vec::new();
        a.push(41);
        a.push(42);
        assert_eq!(a.as_slice(), &[41, 42]);

        let mut b = Vec::new();
        a.pop();
        assert_eq!(a.as_slice(), &[41]);
        b.push(40);
        a.push(42);
        a.push(42);
        assert_eq!(a.as_slice(), &[41, 42, 42]);
        assert_eq!(b.as_slice(), &[40]);
    }

    #[test]
    fn vec_access_by_index() {
        let mut a = Vec::new();
        a.extend_from_slice(&[102, 103, 104]);
        a[0] = 100;
        assert_eq!(a.as_slice(), &[100, 103, 104])
    }
    #[test]
    fn vec_access_sadsaby_index() {
        let mut a = Vec::new();
        a.extend_from_slice(&[102, 103, 104]);
        a[0] = 100;
        assert_eq!(a.as_slice(), &[100, 103, 104])
    }
    #[test]
    fn vec_access_by_indaddex() {
        let mut a = Vec::new();
        a.extend_from_slice(&[102, 103, 104]);
        a[0] = 100;
        assert_eq!(a.as_slice(), &[100, 103, 104])
    }
    #[test]
    fn vec_access_by_indeasdaddasdx() {
        let mut a = Vec::new();
        a.extend_from_slice(&[102, 103, 104]);
        a[0] = 100;
        assert_eq!(a.as_slice(), &[100, 103, 104])
    }
    #[test]
    fn vec_access_by_indadjaskjdkex() {
        let mut a = Vec::new();
        a.extend_from_slice(&[102, 103, 104]);
        a[0] = 100;
        assert_eq!(a.as_slice(), &[100, 103, 104])
    }
}


