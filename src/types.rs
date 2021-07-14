//! This module contains all the headers necessary to parse various aspects of a PE file.
//! Objects taken directly from C are typically prefixed with "Image" and will closely
//! resemble the names of their C counterparts, but named to conform to Rust standards.
//! For example, ```IMAGE_DIRECTORY_ENTRY``` is known as ```ImageDirectoryEntry``` in
//! this library.

use bitflags::bitflags;

use chrono::offset::{Offset as ChronoOffset};
use chrono::offset::TimeZone;
use chrono::{Local as LocalTime};

use std::collections::HashMap;
use std::default::Default;
use std::mem;

use widestring::U16Str;

use crate::{PE, Error};

pub const DOS_SIGNATURE: u16    = 0x5A4D;
pub const OS2_SIGNATURE: u16    = 0x454E;
pub const OS2_SIGNATURE_LE: u16 = 0x454C;
pub const VXD_SIGNATURE: u16    = 0x454C;
pub const NT_SIGNATURE: u32     = 0x00004550;

pub const HDR32_MAGIC: u16 = 0x010B;
pub const HDR64_MAGIC: u16 = 0x020B;
pub const ROM_MAGIC: u16   = 0x0107;

/// Represents the architecture of the PE image.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Arch {
    X86,
    X64,
}

/// Represents a C-style character unit. Basically a wrapper for ```u8```.
#[repr(packed)]
#[derive(Copy, Clone, Default, Eq, PartialEq, Debug)]
pub struct CChar(pub u8);

/* borrowed from pe-rs */
/// Syntactic sugar to get functionality out of C-char referenced slices.
pub trait CCharString {
    /// Get the zero-terminated representation of this string, or ```None``` if it is not zero-terminatedj.
    fn zero_terminated(&self) -> Option<&Self>;
    /// Get the string slice as a ```&str```.
    fn as_str(&self) -> &str;
}
impl CCharString for [CChar] {
    fn zero_terminated(&self) -> Option<&Self> {
        self.iter()
            .position(|&CChar(x)| x == 0)
            .map(|p| &self[..p])
    }
    fn as_str(&self) -> &str {
        let cstr = self.zero_terminated().unwrap_or(&self);

        unsafe { mem::transmute::<&[CChar],&str>(cstr) }
    }
}

/// Represents a UTF16 character unit. Basically a wrapper for ```u16```.
#[repr(packed)]
#[derive(Copy, Clone, Default, Eq, PartialEq, Debug)]
pub struct WChar(pub u16);

/// Syntactic sugar for dealing with UTF16 referenced slices.
pub trait WCharString {
    /// Get the zero-terminated representation of this string, or ```None``` if it is not zero-terminated.
    fn zero_terminated(&self) -> Option<&Self>;
    /// Get the string slice as a ```&U16Str```.
    fn as_u16_str(&self) -> &U16Str;
}
impl WCharString for [WChar] {
    fn zero_terminated(&self) -> Option<&Self> {
        self.iter()
            .position(|&WChar(x)| x == 0)
            .map(|p| &self[..p])
    }
    fn as_u16_str(&self) -> &U16Str {
        let u16str = self.zero_terminated().unwrap_or(&self);

        unsafe { mem::transmute::<&[WChar],&U16Str>(u16str) }
    }
}
/// Represents an object which could be considered an address in a PE file.
pub trait Address {
    /// Convert the address to an offset value.
    fn as_offset(&self, pe: &PE) -> Result<Offset, Error>;
    /// Convert the address to an RVA value.
    fn as_rva(&self, pe: &PE) -> Result<RVA, Error>;
    /// Convert the address to a VA value.
    fn as_va(&self, pe: &PE) -> Result<VA, Error>;
}

/// Represents a file offset in the image. This typically represents an address of the file on disk versus the file in memory.
#[repr(packed)]
#[derive(Copy, Clone, Default, Eq, PartialEq, Debug)]
pub struct Offset(pub u32);
impl Address for Offset {
    fn as_offset(&self, _: &PE) -> Result<Offset, Error> {
        Ok(self.clone())
    }
    fn as_rva(&self, pe: &PE) -> Result<RVA, Error> {
        pe.offset_to_rva(*self)
    }
    fn as_va(&self, pe: &PE) -> Result<VA, Error> {
        pe.offset_to_va(*self)
    }
}

/// Represents a relative virtual address (i.e., RVA). This address typically points to data in memory versus data on disk.
#[repr(packed)]
#[derive(Copy, Clone, Default, Eq, PartialEq, Debug)]
pub struct RVA(pub u32);
impl Address for RVA {
    fn as_offset(&self, pe: &PE) -> Result<Offset, Error> {
        pe.rva_to_offset(*self)
    }
    fn as_rva(&self, _: &PE) -> Result<RVA, Error> {
        Ok(self.clone())
    }
    fn as_va(&self, pe: &PE) -> Result<VA, Error> {
        pe.rva_to_va(*self)
    }
}

/// Represents a 32-bit virtual address (i.e., VA). This address typically points directly to active memory.
#[repr(packed)]
#[derive(Copy, Clone, Default, Eq, PartialEq, Debug)]
pub struct VA32(pub u32);
impl Address for VA32 {
    fn as_offset(&self, pe: &PE) -> Result<Offset, Error> {
        pe.va_to_offset(VA::VA32(*self))
    }
    fn as_rva(&self, pe: &PE) -> Result<RVA, Error> {
        pe.va_to_rva(VA::VA32(*self))
    }
    fn as_va(&self, _: &PE) -> Result<VA, Error> {
        Ok(VA::VA32(self.clone()))
    }
}

/// Represents a 64-bit virtual address (i.e., VA). This address typically points directly to active memory.
#[repr(packed)]
#[derive(Copy, Clone, Default, Eq, PartialEq, Debug)]
pub struct VA64(pub u64);
impl Address for VA64 {
    fn as_offset(&self, pe: &PE) -> Result<Offset, Error> {
        pe.va_to_offset(VA::VA64(*self))
    }
    fn as_rva(&self, pe: &PE) -> Result<RVA, Error> {
        pe.va_to_rva(VA::VA64(*self))
    }
    fn as_va(&self, _: &PE) -> Result<VA, Error> {
        Ok(VA::VA64(self.clone()))
    }
}

/// Represents either a 32-bit or a 64-bit virtual address.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum VA {
    VA32(VA32),
    VA64(VA64),
}
impl Address for VA {
    fn as_offset(&self, pe: &PE) -> Result<Offset, Error> {
        pe.va_to_offset(*self)
    }
    fn as_rva(&self, pe: &PE) -> Result<RVA, Error> {
        pe.va_to_rva(*self)
    }
    fn as_va(&self, _: &PE) -> Result<VA, Error> {
        Ok(self.clone())
    }
}

#[repr(packed)]
pub struct ImageDOSHeader {
    pub e_magic: u16,
    pub e_cblp: u16,
    pub e_cp: u16,
    pub e_crlc: u16,
    pub e_cparhdr: u16,
    pub e_minalloc: u16,
    pub e_maxalloc: u16,
    pub e_ss: u16,
    pub e_sp: u16,
    pub e_csum: u16,
    pub e_ip: u16,
    pub e_cs: u16,
    pub e_lfarlc: u16,
    pub e_ovno: u16,
    pub e_res: [u16; 4],
    pub e_oemid: u16,
    pub e_oeminfo: u16,
    pub e_res2: [u16; 10],
    pub e_lfanew: Offset,
}
impl Default for ImageDOSHeader {
    fn default() -> Self {
        Self {
            e_magic: DOS_SIGNATURE,
            e_cblp: 0x90,
            e_cp: 0x03,
            e_crlc: 0x0,
            e_cparhdr: 0x04,
            e_minalloc: 0x0,
            e_maxalloc: 0xFFFF,
            e_ss: 0x0,
            e_sp: 0xB8,
            e_csum: 0x0,
            e_ip: 0x0,
            e_cs: 0x0,
            e_lfarlc: 0x40,
            e_ovno: 0x0,
            e_res: [0u16; 4],
            e_oemid: 0x0,
            e_oeminfo: 0x0,
            e_res2: [0u16; 10],
            e_lfanew: Offset(0xE0),
        }
    }
}

/// An enum representing the machine values accepted by PE files.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Machine {
    Unknown     = 0x0000,
    TargetHost  = 0x0001,
    I386        = 0x014C,
    R3000       = 0x0162,
    R4000       = 0x0166,
    R10000      = 0x0168,
    WCEMIPSV2   = 0x0169,
    Alpha       = 0x0184,
    SH3         = 0x01A2,
    SH3DSP      = 0x01A3,
    SH3E        = 0x01A4,
    SH4         = 0x01A6,
    SH5         = 0x01A8,
    ARM         = 0x01C0,
    Thumb       = 0x01C2,
    ARMNT       = 0x01C4,
    AM33        = 0x01D3,
    PowerPC     = 0x01F0,
    PowerPCFP   = 0x01F1,
    IA64        = 0x0200,
    MIPS16      = 0x0266,
    Alpha64     = 0x0284,
    MIPSFPU     = 0x0366,
    MIPSFPU16   = 0x0466,
    TRICORE     = 0x0520,
    CEF         = 0x0CEF,
    EBC         = 0x0EBC,
    AMD64       = 0x8664,
    M32R        = 0x9041,
    ARM64       = 0xAA64,
    CEE         = 0xC0EE,
}

bitflags! {
    pub struct FileCharacteristics: u16 {
        const RELOCS_STRIPPED         = 0x0001;
        const EXECUTABLE_IMAGE        = 0x0002;
        const LINE_NUMS_STRIPPED      = 0x0004;
        const LOCAL_SYMS_STRIPPED     = 0x0008;
        const AGGRESSIVE_WS_TRIM      = 0x0010;
        const LARGE_ADDRESS_AWARE     = 0x0020;
        const BYTES_REVERSED_LO       = 0x0080;
        const MACHINE_32BIT           = 0x0100;
        const DEBUG_STRIPPED          = 0x0200;
        const REMOVABLE_RUN_FROM_SWAP = 0x0400;
        const NET_RUN_FROM_SWAP       = 0x0800;
        const SYSTEM                  = 0x1000;
        const DLL                     = 0x2000;
        const UP_SYSTEM_ONLY          = 0x4000;
        const BYTES_REVERSED_HI       = 0x8000;
    }
}

#[repr(packed)]
pub struct ImageFileHeader {
    pub machine: u16,
    pub number_of_sections: u16,
    pub time_date_stamp: u32,
    pub pointer_to_symbol_table: Offset,
    pub number_of_symbols: u32,
    pub size_of_optional_header: u16,
    pub characteristics: FileCharacteristics,
}
impl ImageFileHeader {
    fn default_x86() -> Self {
        ImageFileHeader::default()
    }
    fn default_x64() -> Self {
        Self {
            machine: Machine::AMD64 as u16,
            number_of_sections: 0,
            time_date_stamp: LocalTime.timestamp(0, 0).timestamp() as u32,
            pointer_to_symbol_table: Offset(0),
            number_of_symbols: 0,
            size_of_optional_header: mem::size_of::<ImageOptionalHeader32>() as u16,
            characteristics: FileCharacteristics::EXECUTABLE_IMAGE | FileCharacteristics::MACHINE_32BIT,
        }
    }
}
impl Default for ImageFileHeader {
    fn default() -> Self {
        Self {
            machine: Machine::I386 as u16,
            number_of_sections: 0,
            time_date_stamp: LocalTime.timestamp(0, 0).timestamp() as u32,
            pointer_to_symbol_table: Offset(0),
            number_of_symbols: 0,
            size_of_optional_header: mem::size_of::<ImageOptionalHeader32>() as u16,
            characteristics: FileCharacteristics::EXECUTABLE_IMAGE | FileCharacteristics::MACHINE_32BIT,
        }
    }
}

/// An enum representing the various subsystems possible in a PE file.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Subsystem {
    Unknown                  = 0,
    Native                   = 1,
    WindowsGUI               = 2,
    WindowsCUI               = 3,
    OS2CUI                   = 5,
    POSIXCUI                 = 7,
    NativeWindows            = 8,
    WindowsCEGUI             = 9,
    EFIApplication           = 10,
    EFIBootServiceDriver     = 11,
    EFIRuntimeDriver         = 12,
    EFIROM                   = 13,
    XBox                     = 14,
    WindowsBootApplication   = 16,
    XBoxCodeCatalog          = 17,
}

bitflags! {
    pub struct DLLCharacteristics: u16 {
        const RESERVED1             = 0x0001;
        const RESERVED2             = 0x0002;
        const RESERVED4             = 0x0004;
        const RESERVED8             = 0x0008;
        const HIGH_ENTROPY_VA       = 0x0020;
        const DYNAMIC_BASE          = 0x0040;
        const FORCE_INTEGRITY       = 0x0080;
        const NX_COMPAT             = 0x0100;
        const NO_ISOLATION          = 0x0200;
        const NO_SEH                = 0x0400;
        const NO_BIND               = 0x0800;
        const APPCONTAINER          = 0x1000;
        const WDM_DRIVER            = 0x2000;
        const GUARD_CF              = 0x4000;
        const TERMINAL_SERVER_AWARE = 0x8000;
    }
}

#[repr(packed)]
pub struct ImageOptionalHeader32 {
    pub magic: u16,
    pub major_linker_version: u8,
    pub minor_linker_version: u8,
    pub size_of_code: u32,
    pub size_of_initialized_data: u32,
    pub size_of_uninitialized_data: u32,
    pub address_of_entry_point: RVA,
    pub base_of_code: RVA,
    pub base_of_data: RVA,
    pub image_base: u32,
    pub section_alignment: u32,
    pub file_alignment: u32,
    pub major_operating_system_version: u16,
    pub minor_operating_system_version: u16,
    pub major_image_version: u16,
    pub minor_image_version: u16,
    pub major_subsystem_version: u16,
    pub minor_subsystem_version: u16,
    pub win32_version_value: u32,
    pub size_of_image: u32,
    pub size_of_headers: u32,
    pub checksum: u32,
    pub subsystem: u16,
    pub dll_characteristics: DLLCharacteristics,
    pub size_of_stack_reserve: u32,
    pub size_of_stack_commit: u32,
    pub size_of_heap_reserve: u32,
    pub size_of_heap_commit: u32,
    pub loader_flags: u32,
    pub number_of_rva_and_sizes: u32,
    pub data_directory: [ImageDataDirectory; 16],
}
impl Default for ImageOptionalHeader32 {
    fn default() -> Self {
        Self {
            magic: HDR32_MAGIC,
            major_linker_version: 0xE,
            minor_linker_version: 0x0,
            size_of_code: 0x0,
            size_of_initialized_data: 0x0,
            size_of_uninitialized_data: 0x0,
            address_of_entry_point: RVA(0x1000),
            base_of_code: RVA(0x1000),
            base_of_data: RVA(0),
            image_base: 0x400000,
            section_alignment: 0x1000,
            file_alignment: 0x400,
            major_operating_system_version: 4,
            minor_operating_system_version: 0,
            major_image_version: 4,
            minor_image_version: 0,
            major_subsystem_version: 4,
            minor_subsystem_version: 0,
            win32_version_value: 0,
            size_of_image: 0,
            size_of_headers: 0,
            checksum: 0,
            subsystem: Subsystem::WindowsGUI as u16,
            dll_characteristics: DLLCharacteristics::DYNAMIC_BASE | DLLCharacteristics::NX_COMPAT | DLLCharacteristics::TERMINAL_SERVER_AWARE,
            size_of_stack_reserve: 0x40000,
            size_of_stack_commit: 0x2000,
            size_of_heap_reserve: 0x100000,
            size_of_heap_commit: 0x1000,
            loader_flags: 0,
            number_of_rva_and_sizes: 0x10,
            /* I really don't want to give ImageDataDirectory the copy trait,
               so this is ugly copypasta */
            data_directory: [
                ImageDataDirectory { virtual_address: RVA(0), size: 0 },
                ImageDataDirectory { virtual_address: RVA(0), size: 0 },
                ImageDataDirectory { virtual_address: RVA(0), size: 0 },
                ImageDataDirectory { virtual_address: RVA(0), size: 0 },
                ImageDataDirectory { virtual_address: RVA(0), size: 0 },
                ImageDataDirectory { virtual_address: RVA(0), size: 0 },
                ImageDataDirectory { virtual_address: RVA(0), size: 0 },
                ImageDataDirectory { virtual_address: RVA(0), size: 0 },
                ImageDataDirectory { virtual_address: RVA(0), size: 0 },
                ImageDataDirectory { virtual_address: RVA(0), size: 0 },
                ImageDataDirectory { virtual_address: RVA(0), size: 0 },
                ImageDataDirectory { virtual_address: RVA(0), size: 0 },
                ImageDataDirectory { virtual_address: RVA(0), size: 0 },
                ImageDataDirectory { virtual_address: RVA(0), size: 0 },
                ImageDataDirectory { virtual_address: RVA(0), size: 0 },
                ImageDataDirectory { virtual_address: RVA(0), size: 0 },
            ],
        }
    }
}

#[repr(packed)]
pub struct ImageOptionalHeader64 {
    pub magic: u16,
    pub major_linker_version: u8,
    pub minor_linker_version: u8,
    pub size_of_code: u32,
    pub size_of_initialized_data: u32,
    pub size_of_uninitialized_data: u32,
    pub address_of_entry_point: RVA,
    pub base_of_code: RVA,
    pub image_base: u64,
    pub section_alignment: u32,
    pub file_alignment: u32,
    pub major_operating_system_version: u16,
    pub minor_operating_system_version: u16,
    pub major_image_version: u16,
    pub minor_image_version: u16,
    pub major_subsystem_version: u16,
    pub minor_subsystem_version: u16,
    pub win32_version_value: u32,
    pub size_of_image: u32,
    pub size_of_headers: u32,
    pub checksum: u32,
    pub subsystem: u16,
    pub dll_characteristics: DLLCharacteristics,
    pub size_of_stack_reserve: u64,
    pub size_of_stack_commit: u64,
    pub size_of_heap_reserve: u64,
    pub size_of_heap_commit: u64,
    pub loader_flags: u32,
    pub number_of_rva_and_sizes: u32,
    pub data_directory: [ImageDataDirectory; 16],
}
impl Default for ImageOptionalHeader64 {
    fn default() -> Self {
        Self {
            magic: HDR64_MAGIC,
            major_linker_version: 0xE,
            minor_linker_version: 0x0,
            size_of_code: 0x0,
            size_of_initialized_data: 0x0,
            size_of_uninitialized_data: 0x0,
            address_of_entry_point: RVA(0x1000),
            base_of_code: RVA(0x1000),
            image_base: 0x140000000,
            section_alignment: 0x1000,
            file_alignment: 0x400,
            major_operating_system_version: 6,
            minor_operating_system_version: 0,
            major_image_version: 0,
            minor_image_version: 0,
            major_subsystem_version: 6,
            minor_subsystem_version: 0,
            win32_version_value: 0,
            size_of_image: 0,
            size_of_headers: 0,
            checksum: 0,
            subsystem: Subsystem::WindowsGUI as u16,
            dll_characteristics: DLLCharacteristics::DYNAMIC_BASE | DLLCharacteristics::NX_COMPAT | DLLCharacteristics::TERMINAL_SERVER_AWARE,
            size_of_stack_reserve: 0x100000,
            size_of_stack_commit: 0x1000,
            size_of_heap_reserve: 0x100000,
            size_of_heap_commit: 0x1000,
            loader_flags: 0,
            number_of_rva_and_sizes: 0x10,
            data_directory: [
                ImageDataDirectory { virtual_address: RVA(0), size: 0 },
                ImageDataDirectory { virtual_address: RVA(0), size: 0 },
                ImageDataDirectory { virtual_address: RVA(0), size: 0 },
                ImageDataDirectory { virtual_address: RVA(0), size: 0 },
                ImageDataDirectory { virtual_address: RVA(0), size: 0 },
                ImageDataDirectory { virtual_address: RVA(0), size: 0 },
                ImageDataDirectory { virtual_address: RVA(0), size: 0 },
                ImageDataDirectory { virtual_address: RVA(0), size: 0 },
                ImageDataDirectory { virtual_address: RVA(0), size: 0 },
                ImageDataDirectory { virtual_address: RVA(0), size: 0 },
                ImageDataDirectory { virtual_address: RVA(0), size: 0 },
                ImageDataDirectory { virtual_address: RVA(0), size: 0 },
                ImageDataDirectory { virtual_address: RVA(0), size: 0 },
                ImageDataDirectory { virtual_address: RVA(0), size: 0 },
                ImageDataDirectory { virtual_address: RVA(0), size: 0 },
                ImageDataDirectory { virtual_address: RVA(0), size: 0 },
            ],
        }
    }
}

#[repr(packed)]
pub struct ImageNTHeaders32 {
    pub signature: u32,
    pub file_header: ImageFileHeader,
    pub optional_header: ImageOptionalHeader32,
}
impl Default for ImageNTHeaders32 {
    fn default() -> Self {
        Self {
            signature: NT_SIGNATURE,
            file_header: ImageFileHeader::default_x86(),
            optional_header: ImageOptionalHeader32::default(),
        }
    }
}

#[repr(packed)]
pub struct ImageNTHeaders64 {
    pub signature: u32,
    pub file_header: ImageFileHeader,
    pub optional_header: ImageOptionalHeader64,
}
impl Default for ImageNTHeaders64 {
    fn default() -> Self {
        Self {
            signature: NT_SIGNATURE,
            file_header: ImageFileHeader::default_x64(),
            optional_header: ImageOptionalHeader64::default(),
        }
    }
}

pub enum NTHeaders<'data> {
    NTHeaders32(&'data ImageNTHeaders32),
    NTHeaders64(&'data ImageNTHeaders64),
}

pub enum NTHeadersMut<'data> {
    NTHeaders32(&'data mut ImageNTHeaders32),
    NTHeaders64(&'data mut ImageNTHeaders64),
}

bitflags! {
    pub struct SectionCharacteristics: u32 {
        const TYPE_REG               = 0x00000000;
        const TYPE_DSECT             = 0x00000001;
        const TYPE_NOLOAD            = 0x00000002;
        const TYPE_GROUP             = 0x00000004;
        const TYPE_NO_PAD            = 0x00000008;
        const TYPE_COPY              = 0x00000010;
        const CNT_CODE               = 0x00000020;
        const CNT_INITIALIZED_DATA   = 0x00000040;
        const CNT_UNINITIALIZED_DATA = 0x00000080;
        const LNK_OTHER              = 0x00000100;
        const LNK_INFO               = 0x00000200;
        const TYPE_OVER              = 0x00000400;
        const LNK_REMOVE             = 0x00000800;
        const LNK_COMDAT             = 0x00001000;
        const RESERVED               = 0x00002000;
        const MEM_PROTECTED          = 0x00004000;
        const NO_DEFER_SPEC_EXC      = 0x00004000;
        const GPREL                  = 0x00008000;
        const MEM_FARDATA            = 0x00008000;
        const MEM_SYSHEAP            = 0x00010000;
        const MEM_PURGEABLE          = 0x00020000;
        const MEM_16BIT              = 0x00020000;
        const MEM_LOCKED             = 0x00040000;
        const MEM_PRELOAD            = 0x00080000;
        const ALIGN_1BYTES           = 0x00100000;
        const ALIGN_2BYTES           = 0x00200000;
        const ALIGN_4BYTES           = 0x00300000;
        const ALIGN_8BYTES           = 0x00400000;
        const ALIGN_16BYTES          = 0x00500000;
        const ALIGN_32BYTES          = 0x00600000;
        const ALIGN_64BYTES          = 0x00700000;
        const ALIGN_128BYTES         = 0x00800000;
        const ALIGN_256BYTES         = 0x00900000;
        const ALIGN_512BYTES         = 0x00A00000;
        const ALIGN_1024BYTES        = 0x00B00000;
        const ALIGN_2048BYTES        = 0x00C00000;
        const ALIGN_4096BYTES        = 0x00D00000;
        const ALIGN_8192BYTES        = 0x00E00000;
        const ALIGN_MASK             = 0x00F00000;
        const LNK_NRELOC_OVFL        = 0x01000000;
        const MEM_DISCARDABLE        = 0x02000000;
        const MEM_NOT_CACHED         = 0x04000000;
        const MEM_NOT_PAGED          = 0x08000000;
        const MEM_SHARED             = 0x10000000;
        const MEM_EXECUTE            = 0x20000000;
        const MEM_READ               = 0x40000000;
        const MEM_WRITE              = 0x80000000;
    }
}

#[repr(packed)]
pub struct ImageSectionHeader {
    pub name: [CChar; 8],
    pub virtual_size: u32,
    pub virtual_address: RVA,
    pub size_of_raw_data: u32,
    pub pointer_to_raw_data: Offset,
    pub pointer_to_relocations: Offset,
    pub pointer_to_linenumbers: Offset,
    pub number_of_relocations: u16,
    pub number_of_linenumbers: u16,
    pub characteristics: SectionCharacteristics,
}

#[repr(packed)]
#[derive(Default)]
pub struct ImageDataDirectory {
    pub virtual_address: RVA,
    pub size: u32,
}
impl ImageDataDirectory {
    /// Get the data directory object pointed to by this data directory entry.
    pub fn resolve<'data>(&'data self, pe: &'data PE, entry: ImageDirectoryEntry) -> Result<DataDirectory<'data>, Error> {
        if self.virtual_address.0 == 0 {
            return Err(Error::InvalidRVA);
        }
        
        let address = match self.virtual_address.as_offset(pe) {
            Ok(a) => a,
            Err(e) => return Err(e),
        };

        /* we use an if/else statement instead of a match block for readability */
        if entry == ImageDirectoryEntry::Export {
            match pe.buffer.get_ref::<ImageExportDirectory>(address) {
                Ok(d) => Ok(DataDirectory::Export(d)),
                Err(e) => Err(e),
            }
        }
        else if entry == ImageDirectoryEntry::Import {
            match ImageImportDescriptor::parse_import_table(pe, self) {
                Ok(d) => Ok(DataDirectory::Import(d)),
                Err(e) => Err(e),
            }
        }
        else {
            Err(Error::UnsupportedDirectory)
        }
    }
    /// Get the mutable data directory object pointed to by this data directory entry.
    pub fn resolve_mut<'data>(&'data self, pe: &'data mut PE, entry: ImageDirectoryEntry) -> Result<DataDirectoryMut<'data>, Error> {
        if self.virtual_address.0 == 0 {
            return Err(Error::InvalidRVA);
        }
        
        let address = match self.virtual_address.as_offset(pe) {
            Ok(a) => a,
            Err(e) => return Err(e),
        };

        /* we use an if/else statement instead of a match block for readability */
        if entry == ImageDirectoryEntry::Export {
            match pe.buffer.get_mut_ref::<ImageExportDirectory>(address) {
                Ok(d) => Ok(DataDirectoryMut::Export(d)),
                Err(e) => Err(e),
            }
        }
        else if entry == ImageDirectoryEntry::Import {
            match ImageImportDescriptor::parse_mut_import_table(pe, self) {
                Ok(d) => Ok(DataDirectoryMut::Import(d)),
                Err(e) => Err(e),
            }
        }
        else {
            Err(Error::UnsupportedDirectory)
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ImageDirectoryEntry {
    Export         = 0,
    Import         = 1,
    Resource       = 2,
    Exception      = 3,
    Security       = 4,
    BaseReloc      = 5,
    Debug          = 6,
    Architecture   = 7,
    GlobalPTR      = 8,
    TLS            = 9,
    LoadConfig     = 10,
    BoundImport    = 11,
    IAT            = 12,
    DelayImport    = 13,
    COMDescriptor  = 14,
    Reserved       = 15,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ThunkData {
    ForwarderString(RVA),
    Function(RVA),
    ImportByName(RVA),
    Ordinal(u32),
}
/// Functions to help with thunks in import/export data.
pub trait ThunkFunctions {
    /// Check whether this thunk is an ordinal or not.
    fn is_ordinal(&self) -> bool;
    /// Parse this thunk as an export thunk.
    fn parse_export(&self, start: RVA, end: RVA) -> ThunkData;
    /// Parse this thunk as an import thunk.
    fn parse_import(&self) -> ThunkData;
}

/// Represents a 32-bit thunk entry.
#[repr(packed)]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Thunk32(pub u32);
impl ThunkFunctions for Thunk32 {
    fn is_ordinal(&self) -> bool {
        (self.0 & 0x80000000) != 0
    }
    fn parse_export(&self, start: RVA, end: RVA) -> ThunkData {
        if self.is_ordinal() {
            ThunkData::Ordinal((self.0 & 0xFFFF) as u32)
        }
        else {
            let value = self.0 as u32;

            if start.0 <= value && value < end.0 {
                ThunkData::ForwarderString(RVA(value))
            }
            else {
                ThunkData::Function(RVA(value))
            }
        }
    }
    fn parse_import(&self) -> ThunkData {
        if self.is_ordinal() {
            ThunkData::Ordinal((self.0 & 0xFFFF) as u32)
        }
        else {
            ThunkData::ImportByName(RVA(self.0 as u32))
        }
    }
}

/// Represents a 64-bit thunk entry.
#[repr(packed)]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Thunk64(pub u64);
impl ThunkFunctions for Thunk64 {
    fn is_ordinal(&self) -> bool {
        (self.0 & 0x8000000000000000) != 0
    }
    fn parse_export(&self, start: RVA, end: RVA) -> ThunkData {
        if self.is_ordinal() {
            ThunkData::Ordinal((self.0 & 0xFFFFFFFF) as u32)
        }
        else {
            let value = self.0 as u32;

            if start.0 <= value && value < end.0 {
                ThunkData::ForwarderString(RVA(value))
            }
            else {
                ThunkData::Function(RVA(value))
            }
        }
    }
    fn parse_import(&self) -> ThunkData {
        if self.is_ordinal() {
            ThunkData::Ordinal((self.0 & 0xFFFFFFFF) as u32)
        }
        else {
            ThunkData::ImportByName(RVA(self.0 as u32))
        }
    }
}

/// Abstractly represents a thunk object.
pub enum Thunk<'data> {
    Thunk32(&'data Thunk32),
    Thunk64(&'data Thunk64),
}

/// Abstractly represents a mutable thunk object.
pub enum ThunkMut<'data> {
    Thunk32(&'data mut Thunk32),
    Thunk64(&'data mut Thunk64),
}

#[repr(packed)]
pub struct ImageExportDirectory {
    pub characteristics: u32,
    pub time_date_stamp: u32,
    pub major_version: u16,
    pub minor_version: u16,
    pub name: RVA,
    pub base: u32,
    pub number_of_functions: u32,
    pub number_of_names: u32,
    pub address_of_functions: RVA, // [Thunk32; number_of_functions]
    pub address_of_names: RVA, // [RVA; number_of_names]
    pub address_of_name_ordinals: RVA, // [u16; number_of_names]
}
impl ImageExportDirectory {
    /// Get the name of this export module.
    pub fn get_name<'data>(&self, pe: &'data PE) -> Result<&'data [CChar], Error> {
        if self.name.0 == 0 {
            return Err(Error::InvalidRVA);
        }
        
        match self.name.as_offset(pe) {
            Err(e) => return Err(e),
            Ok(a) => pe.buffer.get_cstring(a, false, None),
        }
    }
    /// Get the mutable name of this export module.
    pub fn get_mut_name<'data>(&self, pe: &'data mut PE) -> Result<&'data mut [CChar], Error> {
        if self.name.0 == 0 {
            return Err(Error::InvalidRVA);
        }

        match self.name.as_offset(pe) {
            Err(e) => return Err(e),
            Ok(a) => pe.buffer.get_mut_cstring(a, false, None),
        }
    }
    /// Get the function array of this export entry. This array represents thunk data pointing to either
    /// ordinals (```ThunkData::Ordinal```), forwarder strings (```ThunkData::ForwarderString```) or
    /// function data (```ThunkData::Function```).
    pub fn get_functions<'data>(&self, pe: &'data PE) -> Result<&'data [Thunk32], Error> {
        if self.address_of_functions.0 == 0 {
            return Err(Error::InvalidRVA);
        }

        match self.address_of_functions.as_offset(pe) {
            Err(e) => return Err(e),
            Ok(a) => pe.buffer.get_slice_ref::<Thunk32>(a, self.number_of_functions as usize),
        }
    }
    /// Get the mutable function array of this export entry.
    pub fn get_mut_functions<'data>(&self, pe: &'data mut PE) -> Result<&'data mut [Thunk32], Error> {
        if self.address_of_functions.0 == 0 {
            return Err(Error::InvalidRVA);
        }

        match self.address_of_functions.as_offset(pe) {
            Err(e) => return Err(e),
            Ok(a) => pe.buffer.get_mut_slice_ref::<Thunk32>(a, self.number_of_functions as usize),
        }
    }
    /// Get the name array of this export entry. This array represents RVA values pointing to zero-terminated
    /// C-style strings.
    pub fn get_names<'data>(&self, pe: &'data PE) -> Result<&'data [RVA], Error> {
        if self.address_of_names.0 == 0 {
            return Err(Error::InvalidRVA);
        }

        match self.address_of_names.as_offset(pe) {
            Err(e) => return Err(e),
            Ok(a) => pe.buffer.get_slice_ref::<RVA>(a, self.number_of_names as usize),
        }
    }
    /// Get the mutable name array of this export entry.
    pub fn get_mut_names<'data>(&self, pe: &'data mut PE) -> Result<&'data mut [RVA], Error> {
        if self.address_of_names.0 == 0 {
            return Err(Error::InvalidRVA);
        }

        match self.address_of_names.as_offset(pe) {
            Err(e) => return Err(e),
            Ok(a) => pe.buffer.get_mut_slice_ref::<RVA>(a, self.number_of_names as usize),
        }
    }
    /// Get the name ordinal array of this export entry. This array mirrors the names array. Values in this
    /// array are indexes into the functions array, representing a name-to-function mapping.
    pub fn get_name_ordinals<'data>(&self, pe: &'data PE) -> Result<&'data [u16], Error> {
        if self.address_of_name_ordinals.0 == 0 {
            return Err(Error::InvalidRVA);
        }

        match self.address_of_name_ordinals.as_offset(pe) {
            Err(e) => return Err(e),
            Ok(a) => pe.buffer.get_slice_ref::<u16>(a, self.number_of_names as usize),
        }
    }
    /// Get the mutable name ordinal array of this export entry.
    pub fn get_mut_name_ordinals<'data>(&self, pe: &'data mut PE) -> Result<&'data mut [u16], Error> {
        if self.address_of_name_ordinals.0 == 0 {
            return Err(Error::InvalidRVA);
        }

        match self.address_of_name_ordinals.as_offset(pe) {
            Err(e) => return Err(e),
            Ok(a) => pe.buffer.get_mut_slice_ref::<u16>(a, self.number_of_names as usize),
        }
    }
    /// Get a mapping of exports to thunk data for this export entry. This maps exported names to thunk data, which can
    /// be an ordinal (```ThunkData::Ordinal```), a function (```ThunkData::Function```) or a forwarder string
    /// (```ThunkData::ForwarderString```).
    pub fn get_export_map<'data>(&self, pe: &'data PE) -> Result<HashMap<&'data str, ThunkData>, Error> {
        let mut result: HashMap<&'data str, ThunkData> = HashMap::<&'data str, ThunkData>::new();

        let directory = match pe.get_data_directory(ImageDirectoryEntry::Export) {
            Ok(d) => d,
            Err(e) => return Err(e),
        };

        let start = directory.virtual_address.clone();
        let end = RVA(start.0 + directory.size);

        let functions = match self.get_functions(pe) {
            Ok(f) => f,
            Err(e) => return Err(e),
        };

        let names = match self.get_names(pe) {
            Ok(n) => n,
            Err(e) => return Err(e),
        };

        let ordinals = match self.get_name_ordinals(pe) {
            Ok(o) => o,
            Err(e) => return Err(e),
        };

        for index in 0u32..self.number_of_names {
            let name_rva = names[index as usize];
            if name_rva.0 == 0 { continue; }

            let name_offset = match name_rva.as_offset(pe) {
                Ok(o) => o,
                Err(_) => continue, /* we continue instead of returning the error to be greedy with parsing */
            };

            let name = match pe.buffer.get_cstring(name_offset, false, None) {
                Ok(s) => s,
                Err(_) => continue,
            };

            let ordinal = ordinals[index as usize];
            let function = functions[ordinal as usize].parse_export(start, end);

            result.insert(name.as_str(), function);
        }

        Ok(result)
    }
}

#[repr(packed)]
pub struct ImageImportDescriptor {
    pub original_first_thunk: RVA,
    pub time_date_stamp: u32,
    pub forwarder_chain: u32,
    pub name: RVA,
    pub first_thunk: RVA,
}
impl ImageImportDescriptor {
    fn parse_thunk_array_size(&self, pe: &PE, rva: RVA) -> Result<usize, Error> {
        if rva.0 == 0 {
            return Err(Error::InvalidRVA);
        }

        let arch = match pe.get_arch() {
            Ok(a) => a,
            Err(e) => return Err(e),
        };
        
        let mut thunks = 0usize;
        let mut indexer = match rva.as_offset(pe) {
            Ok(i) => i,
            Err(e) => return Err(e),
        };

        loop {
            if !pe.validate_offset(indexer) {
                return Err(Error::InvalidOffset);
            }

            match arch {
                Arch::X86 => match pe.buffer.get_ref::<Thunk32>(indexer) {
                    Ok(r) => { if r.0 == 0 { break; } },
                    Err(e) => return Err(e),
                },
                Arch::X64 => match pe.buffer.get_ref::<Thunk64>(indexer) {
                    Ok(r) => { if r.0 == 0 { break; } },
                    Err(e) => return Err(e),
                },
            };

            thunks += 1;
            indexer.0 += match arch {
                Arch::X86 => mem::size_of::<Thunk32>() as u32,
                Arch::X64 => mem::size_of::<Thunk64>() as u32,
            };
        }

        Ok(thunks)
    }
    fn parse_thunk_array<'data>(&self, pe: &'data PE, rva: RVA) -> Result<Vec<Thunk<'data>>, Error> {
        if rva.0 == 0 {
            return Err(Error::InvalidRVA);
        }

        let arch = match pe.get_arch() {
            Ok(a) => a,
            Err(e) => return Err(e),
        };

        let thunks = match self.parse_thunk_array_size(pe, rva) {
            Ok(t) => t,
            Err(e) => return Err(e),
        };

        let offset = match rva.as_offset(pe) {
            Ok(o) => o,
            Err(e) => return Err(e),
        };

        match arch {
            Arch::X86 => match pe.buffer.get_slice_ref::<Thunk32>(offset, thunks) {
                Ok(s) => Ok(s.iter().map(|x| Thunk::Thunk32(x)).collect()),
                Err(e) => Err(e),
            },
            Arch::X64 => match pe.buffer.get_slice_ref::<Thunk64>(offset, thunks) {
                Ok(s) => Ok(s.iter().map(|x| Thunk::Thunk64(x)).collect()),
                Err(e) => Err(e),
            },
        }
    }
    fn parse_mut_thunk_array<'data>(&self, pe: &'data mut PE, rva: RVA) -> Result<Vec<ThunkMut<'data>>, Error> {
        if rva.0 == 0 {
            return Err(Error::InvalidRVA);
        }

        let arch = match pe.get_arch() {
            Ok(a) => a,
            Err(e) => return Err(e),
        };

        let thunks = match self.parse_thunk_array_size(pe, rva) {
            Ok(t) => t,
            Err(e) => return Err(e),
        };

        let offset = match rva.as_offset(pe) {
            Ok(o) => o,
            Err(e) => return Err(e),
        };

        match arch {
            Arch::X86 => match pe.buffer.get_mut_slice_ref::<Thunk32>(offset, thunks) {
                Ok(s) => Ok(s.iter_mut().map(|x| ThunkMut::Thunk32(x)).collect()),
                Err(e) => Err(e),
            },
            Arch::X64 => match pe.buffer.get_mut_slice_ref::<Thunk64>(offset, thunks) {
                Ok(s) => Ok(s.iter_mut().map(|x| ThunkMut::Thunk64(x)).collect()),
                Err(e) => Err(e),
            },
        }
    }

    /// Parse the size of the import table pointed to by the ```ImageDataDirectory``` reference.
    pub fn parse_import_table_size(pe: &PE, dir: &ImageDataDirectory) -> Result<usize, Error> {
        if dir.virtual_address.0 == 0 {
            return Err(Error::InvalidRVA);
        }
        
        let mut address = match dir.virtual_address.as_offset(pe) {
            Ok(a) => a,
            Err(e) => return Err(e),
        };

        let mut imports = 0usize;

        loop {
            if !pe.validate_offset(address) {
                return Err(Error::InvalidOffset);
            }

            match pe.buffer.get_ref::<ImageImportDescriptor>(address) {
                Ok(x) => { if x.original_first_thunk.0 == 0 && x.first_thunk.0 == 0 { break; } },
                Err(e) => return Err(e),
            }

            imports += 1;
            address.0 += mem::size_of::<ImageImportDescriptor>() as u32;
        }

        Ok(imports)
    }
    
    /// Get the import table pointed to by the ```ImageDataDirectory``` reference. This is typically used by the
    /// ```ImageDataDirectory::resolve``` function.
    pub fn parse_import_table<'data>(pe: &'data PE, dir: &'data ImageDataDirectory) -> Result<&'data [ImageImportDescriptor], Error> {
        let size = match ImageImportDescriptor::parse_import_table_size(pe, dir) {
            Ok(s) => s,
            Err(e) => return Err(e),
        };

        let offset = match dir.virtual_address.as_offset(pe) {
            Ok(o) => o,
            Err(e) => return Err(e),
        };

        pe.buffer.get_slice_ref::<ImageImportDescriptor>(offset, size)
    }

    /// Get the mutable import table pointed to by the ```ImageDataDirectory``` reference. This is typically used by the
    /// ```ImageDataDirectory::resolve_mut``` function.
    pub fn parse_mut_import_table<'data>(pe: &'data mut PE, dir: &'data ImageDataDirectory) -> Result<&'data mut [ImageImportDescriptor], Error> {
        let size = match ImageImportDescriptor::parse_import_table_size(pe, dir) {
            Ok(s) => s,
            Err(e) => return Err(e),
        };

        let offset = match dir.virtual_address.as_offset(pe) {
            Ok(o) => o,
            Err(e) => return Err(e),
        };

        pe.buffer.get_mut_slice_ref::<ImageImportDescriptor>(offset, size)
    }

    /// Get the thunk array pointed to by the ```original_first_thunk``` field.
    pub fn get_original_first_thunk<'data>(&self, pe: &'data PE) -> Result<Vec<Thunk<'data>>, Error> {
        self.parse_thunk_array(pe, self.original_first_thunk)
    }
    /// Get the mutable thunk array pointed to by the ```original_first_thunk``` field.
    pub fn get_mut_original_first_thunk<'data>(&self, pe: &'data mut PE) -> Result<Vec<ThunkMut<'data>>, Error> {
        self.parse_mut_thunk_array(pe, self.original_first_thunk)
    }

    /// Get the name of the module represented by this import descriptor entry.
    pub fn get_name<'data>(&self, pe: &'data PE) -> Result<&'data [CChar], Error> {
        let offset = match self.name.as_offset(pe) {
            Ok(o) => o,
            Err(e) => return Err(e),
        };

        pe.buffer.get_cstring(offset, false, None)
    }
    /// Get the mutable name of the module represented by this import descriptor entry.
    pub fn get_mut_name<'data>(&self, pe: &'data mut PE) -> Result<&'data mut [CChar], Error> {
        let offset = match self.name.as_offset(pe) {
            Ok(o) => o,
            Err(e) => return Err(e),
        };

        pe.buffer.get_mut_cstring(offset, false, None)
    }

    /// Get the first thunk array. This array typically represents where in memory imports get resolved to.
    pub fn get_first_thunk<'data>(&self, pe: &'data PE) -> Result<Vec<Thunk<'data>>, Error> {
        self.parse_thunk_array(pe, self.first_thunk)
    }
    /// Get the mutable first thunk array.
    pub fn get_mut_first_thunk<'data>(&self, pe: &'data mut PE) -> Result<Vec<ThunkMut<'data>>, Error> {
        self.parse_mut_thunk_array(pe, self.first_thunk)
    }

    /// Get the imports represented by this import descriptor. This resolves the import table and returns a series of strings
    /// representing both ```ImageImportByName``` structures as well as import ordinals.
    pub fn get_imports(&self, pe: &PE) -> Result<Vec<String>, Error> {
        let mut results = Vec::<String>::new();

        let thunks = match self.get_original_first_thunk(pe) {
            Ok(t) => t,
            Err(e) => {
                if e != Error::InvalidRVA {
                    return Err(e)
                }

                match self.get_first_thunk(pe) {
                    Ok(f) => f,
                    Err(e) => return Err(e),
                }
            },
        };

        for thunk in thunks {
            let thunk_data = match thunk {
                Thunk::Thunk32(t32) => t32.parse_import(),
                Thunk::Thunk64(t64) => t64.parse_import(),
            };

            match thunk_data {
                ThunkData::Ordinal(x) => results.push(String::from(format!("#{}", x))),
                ThunkData::ImportByName(rva) => {
                    let offset = match rva.as_offset(pe) {
                        Ok(o) => o,
                        Err(_) => continue,
                    };
                    
                    match pe.buffer.get_import_by_name(offset) {
                        Ok(i) => results.push(String::from(i.name.as_str())),
                        Err(_) => continue,
                    };
                }
                _ => (),
            }
        }

        Ok(results)
    }
}

/// Represents an ```IMAGE_IMPORT_BY_NAME``` structure.
///
/// ```IMAGE_IMPORT_BY_NAME``` is a variable-sized C structure, which is unsupported in Rust. So, we make
/// a special case for imports by name to try and still retain consistent functionality. This is why
/// this struct is a series of references instead of itself being a raw reference into data.
#[derive(Copy, Clone, Debug)]
pub struct ImageImportByName<'data> {
    pub hint: &'data u16,
    pub name: &'data [CChar],
}

#[repr(u16)]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
/// An enum containing relocation types.
pub enum ImageRelBased {
    Absolute = 0,
    High = 1,
    Low = 2,
    HighLow = 3,
    HighAdj = 4,
    MachineSpecific5 = 5,
    Reserved = 6,
    MachineSpecific7 = 7,
    MachineSpecific8 = 8,
    MachineSpecific9 = 9,
    Dir64 = 10,
    Unknown
}

/// Represents a unit of a relocation, which contains a type and an offset in a ```u16``` value.
#[repr(packed)]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Relocation(pub u16);
impl Relocation {
    /// Get the type of this relocation.
    pub fn get_type(&self) -> ImageRelBased {
        match self.0 >> 12 {
            0 => ImageRelBased::Absolute,
            1 => ImageRelBased::High,
            2 => ImageRelBased::Low,
            3 => ImageRelBased::HighLow,
            4 => ImageRelBased::HighAdj,
            5 => ImageRelBased::MachineSpecific5,
            6 => ImageRelBased::Reserved,
            7 => ImageRelBased::MachineSpecific7,
            8 => ImageRelBased::MachineSpecific8,
            9 => ImageRelBased::MachineSpecific9,
            10 => ImageRelBased::Dir64,
            _ => ImageRelBased::Unknown,
        }
    }
    /// Get the offset of this relocation.
    pub fn get_offset(&self) -> u16 {
        self.0 & 0xFFF
    }
}

/// Represents a parsed relocation entry.
pub struct RelocationEntry<'data> {
    pub base_relocation: &'data ImageBaseRelocation,
    pub relocations: &'data [Relocation]
}

/// Represents a mutable parsed relocation entry.
pub struct RelocationEntryMut<'data> {
    pub base_relocation: &'data mut ImageBaseRelocation,
    pub relocations: &'data mut [Relocation],
}

/// Represents a PE relocation entry.
#[repr(packed)]
pub struct ImageBaseRelocation {
    pub virtual_address: RVA,
    pub size_of_block: u32,
}
impl ImageBaseRelocation {
    /// Get the relocation table pointed to by the given RVA. Returns a vector of ```RelocationEntry``` objects,
    /// which contains the base relocation object as well as the relocation deltas.
    pub fn parse_relocation_table<'data>(pe: &'data PE, dir: &ImageDataDirectory) -> Result<Vec<RelocationEntry<'data>>, Error> {
        if dir.virtual_address.0 == 0 || !pe.validate_rva(dir.virtual_address) {
            return Err(Error::InvalidRVA);
        }

        let mut start_addr = dir.virtual_address.clone();
        let end_addr = RVA(start_addr.0 + dir.size);

        if !pe.validate_rva(end_addr) {
            return Err(Error::InvalidRVA);
        }

        let mut result = Vec::<RelocationEntry>::new();

        while start_addr.0 < end_addr.0 {
            let relocation_size = mem::size_of::<ImageBaseRelocation>();
            let word_size = mem::size_of::<u16>();
            let start_offset = match start_addr.as_offset(pe) {
                Ok(a) => a,
                Err(e) => return Err(e),
            };
            
            let base_relocation = match pe.buffer.get_ref::<ImageBaseRelocation>(start_offset) {
                Ok(b) => b,
                Err(e) => return Err(e),
            };
            
            let block_addr = RVA( ((start_addr.0 as usize) + relocation_size) as u32);
            let block_offset = match block_addr.as_offset(pe) {
                Ok(o) => o,
                Err(e) => return Err(e),
            };
            
            let block_size = ((base_relocation.size_of_block as usize) - relocation_size) / word_size;
            let relocations = match pe.buffer.get_slice_ref::<Relocation>(block_offset, block_size) {
                Ok(r) => r,
                Err(e) => return Err(e),
            };

            result.push(RelocationEntry { base_relocation, relocations });
            start_addr.0 += (relocation_size + (word_size * block_size)) as u32;
        }

        Ok(result)
    }
}

/// An enum representing a data directory object.
///
/// Currently, the following data directories are supported:
/// * ```ImageDirectoryEntry::Export```
/// * ```ImageDirectoryEntry::Import```
///
pub enum DataDirectory<'data> {
    Export(&'data ImageExportDirectory),
    Import(&'data [ImageImportDescriptor]),
    Unsupported,
}

/// An enum representing a mutable data directory object.
pub enum DataDirectoryMut<'data> {
    Export(&'data mut ImageExportDirectory),
    Import(&'data mut [ImageImportDescriptor]),
    Unsupported,
}
