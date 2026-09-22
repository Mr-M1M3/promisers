use mio::{Interest, Token, event::Source};
pub struct Registry<'a, T: Source + ?Sized> {
    pub(crate) id: Token,
    pub(crate) interest: Interest,
    pub(crate) fd: &'a mut T,
}

impl<'a, T: Source + ?Sized> Registry<'a, T> {
    pub fn new(id: usize, fd: &'a mut T, interest: Interest) -> Self {
        Self {
            id: Token(id),
            fd: fd,
            interest,
        }
    }
}
