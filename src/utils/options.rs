pub trait OptExt {
    type T;
    fn unwrap_unreach(self) -> Self::T;
}

impl<T> OptExt for Option<T> {
    type T = T;

    fn unwrap_unreach(self) -> Self::T {
        self.unwrap_or_else(|| unreachable!())
    }
}
impl<T, E> OptExt for color_eyre::Result<T, E> {
    type T = T;

    fn unwrap_unreach(self) -> Self::T {
        self.unwrap_or_else(|_| unreachable!())
    }
}
