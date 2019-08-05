use types::byte_array::ByteArray;
use crate::dispatch::{CostedReturnType, NativeReturnType, Result, StackAccessor};

/// do nothing, just return arg bytes
pub fn native_echo<T: StackAccessor>(mut accessor: T) -> Result<CostedReturnType> {
    let arg = accessor.get_byte_array()?;
    let native_cost = arg.len() as u64;
    let native_return = NativeReturnType::ByteArray(arg);
    Ok(CostedReturnType::new(native_cost, native_return))
}