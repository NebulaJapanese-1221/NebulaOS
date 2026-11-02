use core::ffi::c_void;

#[no_mangle]
pub unsafe extern "C" fn memcpy(
    destination: *mut c_void,
    source: *const c_void,
    count: usize,
) -> *mut c_void {
    let destination_bytes = destination.cast::<u8>();
    let source_bytes = source.cast::<u8>();
    for index in 0..count {
        destination_bytes
            .add(index)
            .write_volatile(source_bytes.add(index).read_volatile());
    }
    destination
}

#[no_mangle]
pub unsafe extern "C" fn memmove(
    destination: *mut c_void,
    source: *const c_void,
    count: usize,
) -> *mut c_void {
    let destination_bytes = destination.cast::<u8>();
    let source_bytes = source.cast::<u8>();
    if (destination_bytes as usize) <= source_bytes as usize {
        for index in 0..count {
            destination_bytes
                .add(index)
                .write_volatile(source_bytes.add(index).read_volatile());
        }
    } else {
        for index in (0..count).rev() {
            destination_bytes
                .add(index)
                .write_volatile(source_bytes.add(index).read_volatile());
        }
    }
    destination
}

#[no_mangle]
pub unsafe extern "C" fn memset(destination: *mut c_void, value: i32, count: usize) -> *mut c_void {
    let bytes = destination.cast::<u8>();
    for index in 0..count {
        bytes.add(index).write_volatile(value as u8);
    }
    destination
}

#[no_mangle]
pub unsafe extern "C" fn memcmp(left: *const c_void, right: *const c_void, count: usize) -> i32 {
    let left = left.cast::<u8>();
    let right = right.cast::<u8>();
    for index in 0..count {
        let left_byte = left.add(index).read_volatile();
        let right_byte = right.add(index).read_volatile();
        if left_byte != right_byte {
            return left_byte as i32 - right_byte as i32;
        }
    }
    0
}
