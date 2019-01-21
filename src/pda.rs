
#[derive(Debug)]
pub enum StackOp<T> {
    None,
    Done,
    Push(T),
    Override(T),
}

#[derive(Debug)]
pub struct StateStack<T> {
    stack: Vec<T>,
}

impl<T> Default for StateStack<T> {
    fn default() -> Self {
        StateStack { stack: vec![] }
    }
}

impl<T> StateStack<T> {
    pub fn new(task: T) -> Self {
        StateStack { stack: vec![task] }
    }

    pub fn transition(&mut self, op: StackOp<T>) {
        match op {
            StackOp::None => {}
            StackOp::Done => {
                self.stack.pop();
            }
            StackOp::Push(task) => self.stack.push(task),
            StackOp::Override(task) => {
                self.stack.clear();
                self.stack.push(task);
            }
        }
    }

    pub fn push(&mut self, item: T) {
        self.stack.push(item)
    }

    pub fn top(&mut self) -> Option<&mut T> {
        self.stack.last_mut()
    }

    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }
}
