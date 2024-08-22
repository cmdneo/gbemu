/// Generate a struct whose fields are bit-fields mapped to the bits of the
/// given underlying type(which should be an unsigned integer).  
/// `new(ux) -> Self`, `read(&self) -> ux` and `write(&mut self, ux)` methods are
/// generated which can be used to create/read/write the struct.  
/// The generated struct implements: `Default`, `Copy` and `Clone` traits.
///
/// A fields is specified as `name: width`, where sum of width of all fields
/// should not exceed the number of bits in the underlying type.  
/// All fields are also of the given underlying type.
/// Fields are mapped from LSB to MSB.
///
/// All the fields and `new`/`read`/`write` methods have visibility as same
/// as that of the struct being defined.
///
/// Underscore(`_`) prefixed field names can be used to ignore bits,
/// for example: `_1: 4`, `_N: 2`.
///
/// **Note**: The struct generated is not space efficient.
///
/// Example:
/// ```no_run
/// bit_mapped!{
///     #[derive(Default)]
///     struct SomeFields<u8> {
///         field1_b3: 3,
///         _skipped: 4,
///         field2_b1: 1,
///     }
/// }
/// ```
macro_rules! bit_fields {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident<$utype:ty> {
            $($(#[$metas:meta])* $fields:ident : $widths:literal),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Default, Copy, Clone)]
        $vis struct $name {
            $($(#[$metas])* $vis $fields: $utype),+
        }

        impl $name {
            #[allow(unused)]
            $vis fn new(v: $utype) -> Self {
                let mut r = Self::default();
                r.write(v);
                r
            }

            #[allow(unused)]
            $vis fn read(&self) -> $utype {
                crate::macros::bit_fields!(@pack
                    $(crate::macros::prefix_field!(self, $fields) ; $widths),+
                )

            }

            $vis fn write(&mut self, v: $utype) {
                crate::macros::bit_fields!(@unpack
                    v,
                    $(crate::macros::prefix_field!(self, $fields) ; $widths),+
                );
            }
        }
    };

    (@unpack $val:expr, $var:expr ; $w:literal) => {
        $var = $val & !(!0 << $w);
    };
    (@unpack $val:expr, $var:expr ; $w:literal, $($vars:expr ; $ws:literal),+) => {
        crate::macros::bit_fields!(@unpack $val, $var ; $w);
        crate::macros::bit_fields!(@unpack $val >> $w, $($vars ; $ws),+)
    };

    (@pack $val:expr ; $w:literal) => { $val & !(!0 << $w)};
    (@pack $val:expr ; $w:literal, $($vals:expr ; $ws:literal),+) => {
        crate::macros::bit_fields!(@pack $val ; $w)
        | (crate::macros::bit_fields!(@pack $($vals ; $ws),+) << $w)
    };
}

/// Concatenates `prefix` with `name` using a dot(`.`).
macro_rules! prefix_field {
    ($prefix:ident, $name:ident) => {
        $prefix.$name
    };
}

/// Match but for type `trait std::ops::RangeBounds<T>`.
/// The match pattern should be a value implementing the trait mentioned.
/// Each arm has an associated action which should be a block-expression.
///
/// If the value is contained in a range then the range offset value
/// =`match_var - *range.start()` is assigned to `bind_name` which is
/// accessible inside the action block and the associated block is run.
///
/// At the end a catch all arm must be present, like so: `_ => { ... }`.
///
/// Example:
/// ```no_run
/// match_ranges! { bind_name@match_var {
///     RANGE1 => { println!("First, offset={bind_name}") }
///     RANGE2 => { println!("Second, ...") }
///     // ...
///     _ => { println!("No match!") }
/// }}
/// ```
macro_rules! match_range {
    ($bind_name:ident@$match_var:ident {
        $($range:expr => $action:block)*
        _ => $final_action:block
    }) => {
        match $match_var {
            $(x if $range.contains(&$match_var) => {
                let $bind_name = x - *$range.start();
                 // Do not warn if unused.
                 // TODO use #expect(unused) when available.
                _ = $bind_name;
                $action
            })*
            _ => $final_action
        }
    };
}

/// Each `range` should have a method `contains(&value) -> bool` for deciding
/// if the value falls withing the range.  
macro_rules! in_ranges {
    ($val:expr, $rng:expr) => {
        $rng.contains(&$val)
    };
    ($val:expr, $rng:expr, $($rngs:expr),+) => {
        $rng.contains(&$val) || in_ranges!($val, $($rngs),+)
    };
}

pub(crate) use bit_fields;
pub(crate) use in_ranges;
pub(crate) use match_range;
pub(crate) use prefix_field;
