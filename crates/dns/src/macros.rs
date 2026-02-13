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

#[cfg(test)]
mod tests {
    // Test enum using the macro
    u16_enum_with_unknown! {
        #[derive(Default)]
        pub enum TestEnum {
            First = 1,
            Second = 2,
            Third = 10,
            Fourth = 100,
        }
    }

    #[test]
    fn test_enum_from_known_values() {
        assert_eq!(TestEnum::from(1), TestEnum::First);
        assert_eq!(TestEnum::from(2), TestEnum::Second);
        assert_eq!(TestEnum::from(10), TestEnum::Third);
        assert_eq!(TestEnum::from(100), TestEnum::Fourth);
    }

    #[test]
    fn test_enum_from_unknown_values() {
        assert_eq!(TestEnum::from(0), TestEnum::Unknown(0));
        assert_eq!(TestEnum::from(3), TestEnum::Unknown(3));
        assert_eq!(TestEnum::from(99), TestEnum::Unknown(99));
        assert_eq!(TestEnum::from(500), TestEnum::Unknown(500));
        assert_eq!(TestEnum::from(u16::MAX), TestEnum::Unknown(u16::MAX));
    }

    #[test]
    fn test_enum_to_u16_known_variants() {
        assert_eq!(TestEnum::First.to_u16(), 1);
        assert_eq!(TestEnum::Second.to_u16(), 2);
        assert_eq!(TestEnum::Third.to_u16(), 10);
        assert_eq!(TestEnum::Fourth.to_u16(), 100);
    }

    #[test]
    fn test_enum_to_u16_unknown_variants() {
        assert_eq!(TestEnum::Unknown(0).to_u16(), 0);
        assert_eq!(TestEnum::Unknown(42).to_u16(), 42);
        assert_eq!(TestEnum::Unknown(999).to_u16(), 999);
        assert_eq!(TestEnum::Unknown(u16::MAX).to_u16(), u16::MAX);
    }

    #[test]
    fn test_enum_roundtrip() {
        // Known values should roundtrip
        assert_eq!(TestEnum::from(TestEnum::First.to_u16()), TestEnum::First);
        assert_eq!(TestEnum::from(TestEnum::Second.to_u16()), TestEnum::Second);
        assert_eq!(TestEnum::from(TestEnum::Third.to_u16()), TestEnum::Third);
        assert_eq!(TestEnum::from(TestEnum::Fourth.to_u16()), TestEnum::Fourth);

        // Unknown values should roundtrip
        let unknown_val = TestEnum::Unknown(42);
        assert_eq!(TestEnum::from(unknown_val.to_u16()), unknown_val);
    }

    #[test]
    fn test_enum_equality() {
        assert_eq!(TestEnum::First, TestEnum::First);
        assert_ne!(TestEnum::First, TestEnum::Second);
        assert_eq!(TestEnum::Unknown(42), TestEnum::Unknown(42));
        assert_ne!(TestEnum::Unknown(42), TestEnum::Unknown(43));
        assert_ne!(TestEnum::First, TestEnum::Unknown(1));
    }

    #[test]
    fn test_enum_copy_clone() {
        let e1 = TestEnum::First;
        let e2 = e1; // Copy
        let e3 = e1.clone(); // Clone
        assert_eq!(e1, e2);
        assert_eq!(e1, e3);

        let u1 = TestEnum::Unknown(99);
        let u2 = u1;
        assert_eq!(u1, u2);
    }

    #[test]
    fn test_enum_debug() {
        let first = format!("{:?}", TestEnum::First);
        assert!(first.contains("First"));

        let unknown = format!("{:?}", TestEnum::Unknown(42));
        assert!(unknown.contains("Unknown"));
        assert!(unknown.contains("42"));
    }

    #[test]
    fn test_enum_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(TestEnum::First);
        set.insert(TestEnum::Second);
        set.insert(TestEnum::Unknown(42));

        assert!(set.contains(&TestEnum::First));
        assert!(set.contains(&TestEnum::Second));
        assert!(set.contains(&TestEnum::Unknown(42)));
        assert!(!set.contains(&TestEnum::Third));
        assert!(!set.contains(&TestEnum::Unknown(43)));
    }

    #[test]
    fn test_boundary_values() {
        // Test minimum value
        let min_unknown = TestEnum::Unknown(0);
        assert_eq!(min_unknown.to_u16(), 0);

        // Test maximum value
        let max_unknown = TestEnum::Unknown(u16::MAX);
        assert_eq!(max_unknown.to_u16(), u16::MAX);
    }
}