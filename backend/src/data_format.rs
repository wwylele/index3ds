use byte_struct::*;

bitfields!(
    #[derive(Debug)]
    pub NcchContentType: u8 {
       pub is_data: 1,
       pub is_executable: 1,
       pub category: 6
    }
);

bitfields!(
    #[derive(Debug)]
    pub NcchKeyConfig: u8 {
        pub fixed_key: 1,
        pub no_romfs: 1,
        pub no_crypto: 1,
        pub reserved_a: 2,
        pub seed_crypto: 1,
        pub reserved_b: 2
    }
);

#[derive(ByteStruct, Debug)]
#[byte_struct_le]
pub struct NcchHeader {
    pub signature: GenericArray<u8, typenum::U256>,
    pub magic: [u8; 4],
    pub content_size: u32,
    pub partition_id: u64,
    pub maker_code: u16,
    pub version: u16,
    pub seed_verifier: [u8; 4],
    pub program_id: u64,
    pub reserved_a: [u8; 16],
    pub logo_hash: [u8; 32],
    pub product_code: [u8; 16],
    pub exheader_hash: [u8; 32],
    pub exheader_size: u32,
    pub reseved_b: [u8; 4],
    pub flag0: u8,
    pub flag1: u8,
    pub flag2: u8,
    pub secondary_key_slot: u8,
    pub platform: u8,
    pub content_type: NcchContentType,
    pub content_unit_size: u8,
    pub key_config: NcchKeyConfig,
    pub sdk_info_offset: u32,
    pub sdk_info_size: u32,
    pub logo_offset: u32,
    pub logo_size: u32,
    pub exefs_offset: u32,
    pub exefs_size: u32,
    pub exefs_hash_region_size: u32,
    pub reserved_c: [u8; 4],
    pub romfs_offset: u32,
    pub romfs_size: u32,
    pub romfs_hash_region_size: u32,
    pub reserved_d: [u8; 4],
    pub exefs_hash: [u8; 32],
    pub romfs_hash: [u8; 32],
}

impl NcchHeader {
    pub fn unit_size(&self) -> usize {
        0x200 * (1 << self.content_unit_size as usize)
    }
}

#[derive(ByteStruct, Debug)]
#[byte_struct_le]
pub struct ExefsFile {
    pub name: [u8; 8],
    pub offset: u32,
    pub size: u32,
}

#[derive(ByteStruct, Debug)]
#[byte_struct_le]
pub struct ExefsHeader {
    pub files: [ExefsFile; 10],
    pub reserved: [u8; 32],
    pub hashes: [[u8; 32]; 10],
}

#[derive(ByteStruct, Debug)]
#[byte_struct_le]
pub struct SmdhTitle {
    pub short: GenericArray<u16, typenum::U64>,
    pub long: GenericArray<u16, typenum::U128>,
    pub publisher: GenericArray<u16, typenum::U64>,
}

#[derive(ByteStruct, Debug)]
#[byte_struct_le]
pub struct Smdh {
    pub magic: [u8; 4],
    pub version: u16,
    pub reserved_a: u16,
    pub title: [SmdhTitle; 16],
    pub ratings: [u8; 16],
    pub region_lockout: u32,
    pub match_maker_id: u32,
    pub match_maker_bit_id: u64,
    pub flags: u32,
    pub eula_version: u16,
    pub reserved_b: u16,
    pub banner_animation_frame: f32,
    pub cec_id: u32,
    pub reserved_c: [u8; 8],
    pub small_icon: GenericArray<u16, typenum::U576>,
    pub large_icon: GenericArray<u16, typenum::Prod<typenum::U64, typenum::U36>>,
}

#[derive(ByteStruct, Debug)]
#[byte_struct_le]
pub struct ExheaderCodeSegment {
    pub address: u32,
    pub num_pages: u32,
    pub code_size: u32,
}

bitfields!(
    #[derive(Debug)]
    pub ExheaderSystemControlFlag: u8 {
        pub compress_code: 1,
        pub sd_app: 1,
        pub reserved: 6,
    }
);

bitfields!(
    #[derive(Debug)]
    pub ExheaderCoreFlag: u32 {
        pub enable_l2_cache: 1,
        pub high_cpu_speed: 1,
        pub reserved_a: 6,
        pub n3ds_system_mode: 4,
        pub reserved_b: 4,
        pub ideal_processor: 2,
        pub affinity_mask: 2,
        pub system_mode: 4,
        pub priority: 8,
    }
);

#[derive(ByteStruct, Debug)]
#[byte_struct_le]
pub struct ExheaderAccessControl {
    pub program_id: u64,
    pub core_version: u32,
    pub core_flag: ExheaderCoreFlag,
    pub resource_limit_desc: [u16; 16],
    pub extdata_id: u64,
    pub system_savedata_id: [u32; 2],
    pub storage_access_id: u64,
    pub filesystem_flag: u64,
    pub services: GenericArray<[u8; 8], typenum::U34>,
    pub reserved_a: [u8; 15],
    pub resource_limit_category: u8,
    pub kernel_desc: [u32; 28],
    pub reserved_b: [u8; 16],
    pub arm9_flag: u32,
    pub arm9_flag_ext: [u8; 11],
    pub arm9_flag_version: u8,
}

#[derive(ByteStruct, Debug)]
#[byte_struct_le]
pub struct Exheader {
    pub name: [u8; 8],
    pub reserved_a: [u8; 5],
    pub system_control_flag: ExheaderSystemControlFlag,
    pub remaster_version: u16,
    pub segment_text: ExheaderCodeSegment,
    pub stack_size: u32,
    pub segment_ro: ExheaderCodeSegment,
    pub reserved_b: [u8; 4],
    pub segment_data: ExheaderCodeSegment,
    pub bss_size: u32,
    pub dependencies: GenericArray<u64, typenum::U48>,
    pub save_data_size: u64,
    pub jump_id: u64,
    pub reserved_c: GenericArray<u8, typenum::U48>,

    pub access_control: ExheaderAccessControl,
    pub signature: GenericArray<u8, typenum::U256>,
    pub public_key: GenericArray<u8, typenum::U256>,
    pub access_control_limit: ExheaderAccessControl,
}

#[test]
fn size_test() {
    assert_eq!(NcchHeader::BYTE_LEN, 0x200);
    assert_eq!(ExefsHeader::BYTE_LEN, 0x200);
    assert_eq!(Smdh::BYTE_LEN, 0x36C0);
    assert_eq!(Exheader::BYTE_LEN, 0x800);
}
