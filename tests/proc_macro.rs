use libhotpatch::hotpatch;

#[hotpatch]
unsafe fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[hotpatch(unsafe)]
unsafe fn add_tuple((a, b): (i32, i32)) -> i32 {
    a + b
}

#[hotpatch]
unsafe fn add_struct(Add { a, b }: Add) -> i32 {
    a + b
}

#[hotpatch]
unsafe fn add_tstruct(Tuple2(a, b): Tuple2<i32, i32>) -> i32 {
    a + b
}

#[hotpatch]
unsafe fn lifetime_bound<'lt>(a: &'lt i32) -> &'lt i32 {
    a
}

#[test]
fn call_add() {
    assert_eq!(unsafe { add(2, 2) }, 4);
}

#[test]
fn call_add_tuple() {
    assert_eq!(unsafe { add_tuple((2, 2)) }, 4);
}

#[test]
fn call_add_struct() {
    assert_eq!(unsafe { add_struct(Add { a: 2, b: 2 }) }, 4);
}

#[test]
fn call_add_tstruct() {
    assert_eq!(unsafe { add_tstruct(Tuple2(2, 2)) }, 4);
}

#[test]
fn call_lifetime_bound() {
    assert_eq!(unsafe { lifetime_bound(&1) }, &1);
}

#[repr(C)]
struct Tuple2<A, B>(A, B);

#[repr(C)]
struct Add {
    a: i32,
    b: i32,
}
