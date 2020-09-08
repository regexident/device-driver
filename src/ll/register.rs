use core::fmt::Debug;

/// General error enum for working with registers
#[derive(Debug)]
pub enum RegisterError<IE: Debug> {
    InvalidValue,
    HardwareError(IE),
}

impl<IE: Debug> From<IE> for RegisterError<IE> {
    fn from(value: IE) -> Self {
        RegisterError::HardwareError(value)
    }
}

/// Trait for reading and writing registers
pub trait RegisterInterface {
    /// The type representation of the address
    type Address;
    /// The type representation of the errors the interface can give
    type InterfaceError: Debug;

    /// Reads the register at the given address and puts the data in the value parameter
    fn read_register(
        &mut self,
        address: Self::Address,
        value: &mut [u8],
    ) -> Result<(), Self::InterfaceError>;

    /// Writes the value to the register at the given address
    fn write_register(
        &mut self,
        address: Self::Address,
        value: &[u8],
    ) -> Result<(), Self::InterfaceError>;
}

#[macro_export]
macro_rules! implement_registers {
    (
        $device_name:ident.$register_set_name:ident<$register_address_type:ty> = {
            $(
                $register_name:ident($register_access_specifier:tt, $register_address:expr, $register_size:expr) = {
                    $(
                        $field_name:ident: $field_type:ty = $field_access_specifier:tt $field_bit_range:expr
                    ),* $(,)?
                }
            ),* $(,)?
        }
    ) => {
        pub mod $register_set_name {
            use super::*;
            use device_driver::ll::register::RegisterInterface;
            use device_driver::ll::LowLevelDevice;
            use device_driver::implement_register;
            use device_driver::implement_register_field;

            impl<'a, I> $device_name<I>
            where
                I: 'a + RegisterInterface<Address = $register_address_type>,
            {
                pub fn $register_set_name(&'a mut self) -> RegisterSet<'a, I> {
                    RegisterSet::new(&mut self.interface)
                }
            }

            /// A struct that borrows the interface from the device.
            /// It implements the read and/or write functionality for the registers.
            pub struct RegAccessor<'a, I, R, W>
            where
                I: 'a + RegisterInterface<Address = $register_address_type>,
            {
                interface: &'a mut I,
                phantom: core::marker::PhantomData<(R, W)>,
            }

            impl<'a, I, R, W> RegAccessor<'a, I, R, W>
            where
                I: 'a + RegisterInterface<Address = $register_address_type>,
            {
                fn new(interface: &'a mut I) -> Self {
                    Self {
                        interface,
                        phantom: Default::default(),
                    }
                }
            }

            /// A struct containing all the register definitions
            pub struct RegisterSet<'a, I>
            where
                I: 'a + RegisterInterface<Address = $register_address_type>,
            {
                interface: &'a mut I,
            }

            impl<'a, I> RegisterSet<'a, I>
            where
                I: 'a + RegisterInterface<Address = $register_address_type>,
            {
                fn new(interface: &'a mut I) -> Self {
                    Self { interface }
                }

                $(
                    pub fn $register_name(&'a mut self) -> RegAccessor<'a, I, $register_name::R, $register_name::W> {
                        RegAccessor::new(&mut self.interface)
                    }
                )*
            }

            $(
                pub mod $register_name {
                    use super::*;

                    implement_register!(
                        ($register_access_specifier, $register_address, $register_size, $register_address_type) {
                            $(
                                $field_name: $field_type = $field_access_specifier $field_bit_range
                            ),*
                        }
                    );
                }
            )*
        }
    };
}

#[macro_export]
macro_rules! implement_register {
    ((@R, $register_address:expr, $register_size:expr, $register_address_type:ty) {
            $(
                $field_name:ident: $field_type:ty = $field_access_specifier:tt $field_bit_range:expr
            ),*
        }
    ) => {
        pub struct R([u8; $register_size]);

        impl R {
            fn zero() -> Self {
                Self([0; $register_size])
            }

            $(
                implement_register_field!(@R, $field_name: $field_type = $field_access_specifier $field_bit_range);
            )*
        }

        impl<'a, I> RegAccessor<'a, I, R, W>
        where
            I: RegisterInterface<Address = $register_address_type>,
        {
            /// Reads the register
            pub fn read(&mut self) -> Result<R, RegisterError<I::InterfaceError>> {
                let mut r = R::zero();
                self.interface.read_register($register_address, &mut r.0)?;
                Ok(r)
            }
        }
    };
    ((@W, $register_address:expr, $register_size:expr, $register_address_type:ty) {
            $(
                $field_name:ident: $field_type:ty = $field_access_specifier:tt $field_bit_range:expr
            ),*
        }
    ) => {
        pub struct W([u8; $register_size]);

        impl W {
            fn zero() -> Self {
                Self([0; $register_size])
            }

            $(
                implement_register_field!(@W, $field_name: $field_type = $field_access_specifier $field_bit_range);
            )*
        }

        impl<'a, I> RegAccessor<'a, I, R, W>
        where
            I: RegisterInterface<Address = $register_address_type>,
        {
            /// Writes the value returned by the closure to the register
            pub fn write<F>(&mut self, f: F) -> Result<(), RegisterError<I::InterfaceError>>
            where
                F: FnOnce(W) -> W,
            {
                let w = f(W::zero());
                self.interface.write_register($register_address, &w.0)?;
                Ok(())
            }
        }
    };
    ((RW, $register_address:expr, $register_size:expr, $register_address_type:ty) {
            $(
                $field_name:ident: $field_type:ty = $field_access_specifier:tt $field_bit_range:expr
            ),*
        }
    ) => {
        implement_register!(
            (@R, $register_address, $register_size, $register_address_type) {
                $(
                    $field_name: $field_type = $field_access_specifier $field_bit_range
                ),*
            }
        );
        implement_register!(
            (@W, $register_address, $register_size, $register_address_type) {
                $(
                    $field_name: $field_type = $field_access_specifier $field_bit_range
                ),*
            }
        );

        impl<'a, I> RegAccessor<'a, I, R, W>
        where
            I: RegisterInterface<Address = $register_address_type>,
        {
            /// Reads the register, gives the value to the closure and writes back the value returned by the closure
            pub fn modify<F>(&mut self, f: F) -> Result<(), RegisterError<I::InterfaceError>>
            where
                F: FnOnce(R, W) -> W,
            {
                let r = self.read()?;
                let w = W(r.0.clone());

                let w = f(r, w);

                self.write(|_| w)?;
                Ok(())
            }
        }
    };
    ((RO, $register_address:expr, $register_size:expr, $register_address_type:ty) {
            $(
                $field_name:ident: $field_type:ty = $field_access_specifier:tt $field_bit_range:expr
            ),*
        }
    ) => {
        implement_register!(
            (@R, $register_address, $register_size, $register_address_type) {
                $(
                    $field_name: $field_type = $field_access_specifier $field_bit_range
                ),*
            }
        );

        pub type W = ();
    };
    ((WO, $register_address:expr, $register_size:expr, $register_address_type:ty) {
            $(
                $field_name:ident: $field_type:ty = $field_access_specifier:tt $field_bit_range:expr
            ),*
        }
    ) => {
        implement_register!(
            (@W, $register_address, $register_size, $register_address_type) {
                $(
                    $field_name: $field_type = $field_access_specifier $field_bit_range
                ),*
            }
        );

        pub type R = ();
    };
}

#[macro_export]
macro_rules! implement_register_field {
    (@R, $field_name:ident: $field_type:ty = RO $field_bit_range:expr) => {
        pub fn $field_name(&self) -> $field_type {
            use bitvec::prelude::*;
            use bitvec::view::AsBits;

            self.0.as_bits::<Lsb0>()[$field_bit_range].load_be()
        }
    };
    (@R, $field_name:ident: $field_type:ty = WO $field_bit_range:expr) => {
        // Empty on purpose
    };
    (@R, $field_name:ident: $field_type:ty = RW $field_bit_range:expr) => {
        implement_register_field!(@R, $field_name: $field_type = RO $field_bit_range);
        implement_register_field!(@R, $field_name: $field_type = WO $field_bit_range);
    };
    (@W, $field_name:ident: $field_type:ty = RO $field_bit_range:expr) => {
        // Empty on purpose
    };
    (@W, $field_name:ident: $field_type:ty = WO $field_bit_range:expr) => {
        pub fn $field_name(mut self, value: $field_type) -> Self {
            use bitvec::prelude::*;
            use bitvec::view::AsBitsMut;

            self.0.as_bits_mut::<Lsb0>()[$field_bit_range].store_be(value);

            self
        }
    };
    (@W, $field_name:ident: $field_type:ty = RW $field_bit_range:expr) => {
        implement_register_field!(@W, $field_name: $field_type = RO $field_bit_range);
        implement_register_field!(@W, $field_name: $field_type = WO $field_bit_range);
    };
}