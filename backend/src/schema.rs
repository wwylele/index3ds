table! {
    ncch (id) {
        id -> Text,
        ncch_signature -> Bytea,
        content_size -> Int4,
        partition_id -> Int8,
        maker_code -> Int2,
        ncch_verson -> Int2,
        program_id -> Int8,
        product_code -> Bytea,
        secondary_key_slot -> Int2,
        platform -> Int2,
        content_is_data -> Bool,
        content_is_executable -> Bool,
        content_category -> Int2,
        content_unit_size -> Int2,
        fixed_key -> Bool,
        no_romfs -> Bool,
        no_crypto -> Bool,
        seed_crypto -> Bool,
        exheader_name -> Nullable<Bytea>,
        sd_app -> Nullable<Bool>,
        remaster_version -> Nullable<Int2>,
        dependencies -> Nullable<Array<Int8>>,
        save_data_size -> Nullable<Int8>,
        jump_id -> Nullable<Int8>,
        exheader_program_id -> Nullable<Int8>,
        core_version -> Nullable<Int4>,
        enable_l2_cache -> Nullable<Bool>,
        high_cpu_speed -> Nullable<Bool>,
        system_mode -> Nullable<Int2>,
        n3ds_system_mode -> Nullable<Int2>,
        ideal_processor -> Nullable<Int2>,
        affinity_mask -> Nullable<Int2>,
        thread_priority -> Nullable<Int2>,
        resource_limit_desc -> Nullable<Array<Int2>>,
        extdata_id -> Nullable<Int8>,
        system_savedata_id0 -> Nullable<Int4>,
        system_savedata_id1 -> Nullable<Int4>,
        storage_access_id -> Nullable<Int8>,
        filesystem_flag -> Nullable<Int8>,
        services -> Nullable<Array<Bytea>>,
        resource_limit_category -> Nullable<Int2>,
        kernel_desc -> Nullable<Array<Int4>>,
        arm9_flag -> Nullable<Int4>,
        arm9_flag_version -> Nullable<Int2>,
        short_title -> Nullable<Array<Int2>>,
        long_title -> Nullable<Array<Int2>>,
        publisher -> Nullable<Array<Int2>>,
        ratings -> Nullable<Array<Int2>>,
        region_lockout -> Nullable<Int4>,
        match_maker_id -> Nullable<Int4>,
        match_maker_bit_id -> Nullable<Int8>,
        smdh_flags -> Nullable<Int4>,
        eula_version -> Nullable<Int2>,
        cec_id -> Nullable<Int4>,
        small_icon -> Nullable<Array<Int2>>,
        large_icon -> Nullable<Array<Int2>>,
        keyword -> Text,
    }
}