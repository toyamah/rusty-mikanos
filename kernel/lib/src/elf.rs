use core::mem;

const EI_NIDENT: usize = 16;

type Elf64Addr = usize;
type Elf64Off = u64;
type Elf64Half = u16;
type Elf64Word = u32;
type Elf64Sword = i32;
type Elf64Xword = u64;
type Elf64Sxword = i64;

#[repr(C)]
pub struct Elf64Ehdr {
    pub e_ident: [u8; EI_NIDENT],
    pub e_type: Elf64Half,
    pub e_machine: Elf64Half,
    pub e_version: Elf64Word,
    pub e_entry: Elf64Addr,
    pub e_phoff: Elf64Off,
    pub e_shoff: Elf64Off,
    pub e_flags: Elf64Word,
    pub e_ehsize: Elf64Half,
    pub e_phentsize: Elf64Half,
    pub e_phnum: Elf64Half,
    pub e_shentsize: Elf64Half,
    pub e_shnum: Elf64Half,
    pub e_shstrndx: Elf64Half,
}

impl Elf64Ehdr {
    pub(crate) unsafe fn from(file_buf: &[u8]) -> Option<&Elf64Ehdr> {
        let header_size = mem::size_of::<Elf64Ehdr>();
        let elf_header = &file_buf[..header_size] as *const _ as *const Elf64Ehdr;
        elf_header.as_ref()
    }

    pub(crate) fn is_elf(&self) -> bool {
        &self.e_ident[..4] == b"\x7fELF"
    }
}
