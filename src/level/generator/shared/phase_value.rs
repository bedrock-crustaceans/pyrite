#[macro_export]
macro_rules! phase_value_enum {
    ($vis:vis enum $name:ident for $generator:ty { $($variant:ident($phase:ty) => $ty:ty),+ $(,)? }) => {
        #[derive(Clone)]
        $vis enum $name {
            $($variant(std::sync::Arc<$ty>)),+
        }

        $(
            impl chorus::level::generator::phase::PhaseValue<$generator> for $phase {
                fn wrap(output: std::sync::Arc<Self::Output>) -> $name {
                    $name::$variant(output)
                }

                fn unwrap(value: &$name) -> Option<std::sync::Arc<Self::Output>> {
                    match value {
                        $name::$variant(v) => Some(std::sync::Arc::clone(v)),
                        _ => None,
                    }
                }
            }
        )+
    };
}
