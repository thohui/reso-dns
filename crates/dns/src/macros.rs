#[macro_export]
macro_rules! u16_enum_with_unknown {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $(
                $(#[$vmeta:meta])*
                $variant:ident = $value:expr
            ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
        $vis enum $name {
            $(
                $(#[$vmeta])*
                $variant,
            )*
            Unknown(u16),
        }

        impl $name {
            pub const fn to_u16(self) -> u16 {
                match self {
                    $(Self::$variant => $value,)*
                    Self::Unknown(v) => v,
                }
            }
        }

        impl From<u16> for $name {
            fn from(v: u16) -> Self {
                match v {
                    $($value => Self::$variant,)*
                    other => Self::Unknown(other),
                }
            }
        }
    };
}
