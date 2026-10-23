#[derive(Clone)]
pub(crate) struct Output(pub(crate) std::rc::Rc<std::cell::RefCell<Vec<u8>>>);

impl std::io::Write for Output {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
