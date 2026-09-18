pub enum PromiseState<T, E> {
    Pending,
    Resolved(T),
    Rejected(E),
    Done
}

impl<T:'static, E: 'static> PromiseState<T, E> {
}
