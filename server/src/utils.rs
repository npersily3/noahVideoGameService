use std::collections::VecDeque;

struct RingBuffer<T,> {
    array: Vec<Option<T>>,
    front: usize,
    tail: usize,
    len: usize,
}

impl<T,> RingBuffer<T,> {

    fn new(capacity: Option<usize>) -> RingBuffer<T,> {
        let len = capacity.unwrap_or(32);

        RingBuffer {
            array: (0..len).map(|_| None).collect(),
            front: 0,
            tail: 0,
            len
        }
    }

    fn push_front(&mut self, value: T) {
        self.front = (self.front + 1) % self.array.len();
        self.array[self.front] = Some(value);
    }
    fn peak(&self) -> Option<&T> {
        let option = self.array.get(self.front);
        match option {
            None => None,
            Some(value) => {
                Option::from(value)
            }
        }
    }

}