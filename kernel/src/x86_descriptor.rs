/// A file that contains shared definitions for Segment and Interrupt Descriptor

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SystemDescriptorType {
    Upper8Bytes = 0,
    LDT = 2,
    TSSAvailable = 9,
    TSSBusy = 11,
    CallGate = 12,
    InterruptGate = 14,
    TrapGate = 15,
}

/// Define a new type because the same values cannot be assigned in Rust.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SegmentDescriptorType {
    // code & data segment types
    ReadWrite = 2,
    ExecuteRead = 10,
}
