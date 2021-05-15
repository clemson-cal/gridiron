#[repr(C)]
pub struct Status {
    pub count: i32,
    pub source: i32,
    pub tag: i32,
}

extern {
    pub fn init() -> i32;
    pub fn finalize();
    pub fn comm_rank() -> i32;
    pub fn comm_size() -> i32;
    pub fn send(buf: *const u8, count: i32, dest: i32, tag: i32);
    pub fn recv(buf: *mut u8, count: i32, source: i32, tag: i32);
    pub fn probe_tag(tag: i32) -> Status;
}
