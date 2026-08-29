pub unsafe fn abs(x: i32) -> i32 {
    if x < 0 { -x } else { x }
}

pub unsafe fn labs(x: i64) -> i64 {
    if x < 0 { -x } else { x }
}

pub unsafe fn min(a: i32, b: i32) -> i32 {
    if a < b { a } else { b }
}

pub unsafe fn max(a: i32, b: i32) -> i32 {
    if a > b { a } else { b }
}

pub unsafe fn lmin(a: i64, b: i64) -> i64 {
    if a < b { a } else { b }
}

pub unsafe fn lmax(a: i64, b: i64) -> i64 {
    if a > b { a } else { b }
}

pub unsafe fn fabs(x: f32) -> f32 {
    if x < 0.0 { -x } else { x }
}

pub unsafe fn fabsl(x: f64) -> f64 {
    if x < 0.0 { -x } else { x }
}

pub unsafe fn isqrt(x: i32) -> i32 {
    if x <= 0 {
        return 0;
    }
    let mut r = x;
    let mut y = 1;
    while r > y {
        r = (r + y) / 2;
        y = x / r;
    }
    r
}

pub unsafe fn srand(_seed: u32) {}

pub unsafe fn rand() -> i32 {
    1
}
