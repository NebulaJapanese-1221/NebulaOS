pub unsafe fn isdigit(c: i32) -> i32 {
    if (c as u8) >= b'0' && (c as u8) <= b'9' { 1 } else { 0 }
}

pub unsafe fn isalpha(c: i32) -> i32 {
    let c = c as u8;
    if (c >= b'a' && c <= b'z') || (c >= b'A' && c <= b'Z') { 1 } else { 0 }
}

pub unsafe fn isalnum(c: i32) -> i32 {
    if isdigit(c) != 0 || isalpha(c) != 0 { 1 } else { 0 }
}

pub unsafe fn isspace(c: i32) -> i32 {
    match c as u8 {
        b' ' | b'\t' | b'\n' | b'\r' | b'\x0B' | b'\x0C' => 1,
        _ => 0,
    }
}

pub unsafe fn isupper(c: i32) -> i32 {
    if (c as u8) >= b'A' && (c as u8) <= b'Z' { 1 } else { 0 }
}

pub unsafe fn islower(c: i32) -> i32 {
    if (c as u8) >= b'a' && (c as u8) <= b'z' { 1 } else { 0 }
}

pub unsafe fn toupper(c: i32) -> i32 {
    let c = c as u8;
    if c >= b'a' && c <= b'z' { (c - b'a' + b'A') as i32 } else { c as i32 }
}

pub unsafe fn tolower(c: i32) -> i32 {
    let c = c as u8;
    if c >= b'A' && c <= b'Z' { (c - b'A' + b'a') as i32 } else { c as i32 }
}
