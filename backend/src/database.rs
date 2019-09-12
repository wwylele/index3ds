use crate::api::*;
use crate::data_format::*;
use crate::schema::*;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::*;
use diesel::result::{DatabaseErrorKind, Error};
use diesel::{Insertable, Queryable};
use log::{error, info, warn};
use std::env;

#[derive(Queryable, Insertable, Debug)]
#[table_name = "ncch"]
pub struct NcchRecord {
    pub id: String,
    pub ncch_signature: Vec<u8>,
    content_size: i32,
    partition_id: i64,
    maker_code: i16,
    ncch_verson: i16,
    program_id: i64,
    product_code: Vec<u8>,
    secondary_key_slot: i16,
    platform: i16,
    content_is_data: bool,
    content_is_executable: bool,
    content_category: i16,
    content_unit_size: i16,
    fixed_key: bool,
    no_romfs: bool,
    no_crypto: bool,
    seed_crypto: bool,

    exheader_name: Option<Vec<u8>>,
    sd_app: Option<bool>,
    remaster_version: Option<i16>,
    dependencies: Option<Vec<i64>>,
    save_data_size: Option<i64>,
    jump_id: Option<i64>,
    exheader_program_id: Option<i64>,
    core_version: Option<i32>,
    enable_l2_cache: Option<bool>,
    high_cpu_speed: Option<bool>,
    system_mode: Option<i16>,
    n3ds_system_mode: Option<i16>,
    ideal_processor: Option<i16>,
    affinity_mask: Option<i16>,
    thread_priority: Option<i16>,
    resource_limit_desc: Option<Vec<i16>>,
    extdata_id: Option<i64>,
    system_savedata_id0: Option<i32>,
    system_savedata_id1: Option<i32>,
    storage_access_id: Option<i64>,
    filesystem_flag: Option<i64>,
    services: Option<Vec<Vec<u8>>>,
    resource_limit_category: Option<i16>,
    kernel_desc: Option<Vec<i32>>,
    arm9_flag: Option<i32>,
    arm9_flag_version: Option<i16>,

    short_title: Option<Vec<i16>>,
    long_title: Option<Vec<i16>>,
    publisher: Option<Vec<i16>>,
    ratings: Option<Vec<i16>>,
    region_lockout: Option<i32>,
    match_maker_id: Option<i32>,
    match_maker_bit_id: Option<i64>,
    smdh_flags: Option<i32>,
    eula_version: Option<i16>,
    cec_id: Option<i32>,
    pub small_icon: Option<Vec<i16>>,
    pub large_icon: Option<Vec<i16>>,

    keyword: String,
}

fn trim<'a, U, T: PartialEq<U>>(to_trim: &U, mut s: &'a [T]) -> &'a [T] {
    while s.last().map(|s| *s == *to_trim).unwrap_or(false) {
        s = &s[0..s.len() - 1]
    }
    s
}

fn normalize(text: &str) -> String {
    text.chars()
        .map(|c| {
            let c = c.to_ascii_lowercase();
            if c == 'é' {
                'e'
            } else if c == '\n' {
                ' '
            } else {
                c
            }
        })
        .collect()
}

fn convert_string(s: &[u8]) -> String {
    trim(&0, s).iter().map(|&x| x as char).collect()
}

fn convert_string16(s: &[u16]) -> String {
    String::from_utf16_lossy(trim(&0, s))
}

fn generate_keyword(
    header: &NcchHeader,
    exheader: Option<&Exheader>,
    smdh: Option<&Smdh>,
) -> String {
    let mut soup = std::collections::HashSet::<String>::new();
    soup.insert(normalize(&format!("{:016x}", header.partition_id)));
    soup.insert(normalize(&format!("{:016x}", header.program_id)));
    soup.insert(normalize(&convert_string(&header.product_code)));

    if let Some(exheader) = exheader {
        soup.insert(normalize(&format!(
            "{:016x}",
            exheader.access_control.program_id
        )));
        soup.insert(normalize(&convert_string(&exheader.name)));
    }

    if let Some(smdh) = smdh {
        for title in smdh.title.iter() {
            soup.insert(normalize(&convert_string16(&title.short)));
            soup.insert(normalize(&convert_string16(&title.long)));
            soup.insert(normalize(&convert_string16(&title.publisher)));
        }
    }

    soup.into_iter()
        .fold("".to_owned(), |b, x| format!("{}{}\n", b, x))
}

impl NcchRecord {
    #[allow(clippy::cast_lossless)]
    pub fn new(header: NcchHeader, exheader: Option<Exheader>, smdh: Option<Smdh>) -> NcchRecord {
        let keyword = generate_keyword(&header, exheader.as_ref(), smdh.as_ref());
        let id = format!(
            "{:016x}-{}",
            header.partition_id,
            header.signature[..]
                .iter()
                .take(16)
                .map(|c| format!("{:02x}", c))
                .fold("".to_owned(), |mut a, x| {
                    a.push_str(&x);
                    a
                })
        );
        let exheader = exheader.as_ref();
        let smdh = smdh.as_ref();
        NcchRecord {
            id,
            ncch_signature: header.signature[..].to_vec(),
            content_size: header.content_size as i32,
            partition_id: header.partition_id as i64,
            maker_code: header.maker_code as i16,
            ncch_verson: header.version as i16,
            program_id: header.program_id as i64,
            product_code: header.product_code.to_vec(),
            secondary_key_slot: header.secondary_key_slot as i16,
            platform: header.platform as i16,
            content_is_data: header.content_type.is_data != 0,
            content_is_executable: header.content_type.is_executable != 0,
            content_category: header.content_type.category as i16,
            content_unit_size: header.content_unit_size as i16,
            fixed_key: header.key_config.fixed_key != 0,
            no_romfs: header.key_config.no_romfs != 0,
            no_crypto: header.key_config.no_crypto != 0,
            seed_crypto: header.key_config.seed_crypto != 0,

            exheader_name: exheader.map(|e| e.name.to_vec()),
            sd_app: exheader.map(|e| e.system_control_flag.sd_app != 0),
            remaster_version: exheader.map(|e| e.remaster_version as i16),
            dependencies: exheader.map(|e| e.dependencies[..].iter().map(|&i| i as i64).collect()),
            save_data_size: exheader.map(|e| e.save_data_size as i64),
            jump_id: exheader.map(|e| e.jump_id as i64),
            exheader_program_id: exheader.map(|e| e.access_control.program_id as i64),
            core_version: exheader.map(|e| e.access_control.core_version as i32),
            enable_l2_cache: exheader.map(|e| e.access_control.core_flag.enable_l2_cache != 0),
            high_cpu_speed: exheader.map(|e| e.access_control.core_flag.high_cpu_speed != 0),
            system_mode: exheader.map(|e| e.access_control.core_flag.system_mode as i16),
            n3ds_system_mode: exheader.map(|e| e.access_control.core_flag.n3ds_system_mode as i16),
            ideal_processor: exheader.map(|e| e.access_control.core_flag.ideal_processor as i16),
            affinity_mask: exheader.map(|e| e.access_control.core_flag.affinity_mask as i16),
            thread_priority: exheader.map(|e| e.access_control.core_flag.priority as i16),
            resource_limit_desc: exheader.map(|e| {
                e.access_control
                    .resource_limit_desc
                    .iter()
                    .map(|&x| x as i16)
                    .collect()
            }),
            extdata_id: exheader.map(|e| e.access_control.extdata_id as i64),
            system_savedata_id0: exheader.map(|e| e.access_control.system_savedata_id[0] as i32),
            system_savedata_id1: exheader.map(|e| e.access_control.system_savedata_id[1] as i32),
            storage_access_id: exheader.map(|e| e.access_control.storage_access_id as i64),
            filesystem_flag: exheader.map(|e| e.access_control.filesystem_flag as i64),
            services: exheader.map(|e| {
                e.access_control.services[..]
                    .iter()
                    .map(|i| i.to_vec())
                    .collect()
            }),
            resource_limit_category: exheader
                .map(|e| e.access_control.resource_limit_category as i16),
            kernel_desc: exheader.map(|e| {
                e.access_control
                    .kernel_desc
                    .iter()
                    .map(|&x| x as i32)
                    .collect()
            }),
            arm9_flag: exheader.map(|e| e.access_control.arm9_flag as i32),
            arm9_flag_version: exheader.map(|e| e.access_control.arm9_flag_version as i16),

            short_title: smdh.map(|s| {
                s.title
                    .iter()
                    .map(|t| t.short[..].iter().map(|&c| c as i16))
                    .flatten()
                    .collect()
            }),
            long_title: smdh.map(|s| {
                s.title
                    .iter()
                    .map(|t| t.long[..].iter().map(|&c| c as i16))
                    .flatten()
                    .collect()
            }),
            publisher: smdh.map(|s| {
                s.title
                    .iter()
                    .map(|t| t.publisher[..].iter().map(|&c| c as i16))
                    .flatten()
                    .collect()
            }),
            ratings: smdh.map(|s| s.ratings.iter().map(|&r| r as i16).collect()),
            region_lockout: smdh.map(|s| s.region_lockout as i32),
            match_maker_id: smdh.map(|s| s.match_maker_id as i32),
            match_maker_bit_id: smdh.map(|s| s.match_maker_bit_id as i64),
            smdh_flags: smdh.map(|s| s.flags as i32),
            eula_version: smdh.map(|s| s.eula_version as i16),
            cec_id: smdh.map(|s| s.cec_id as i32),
            small_icon: smdh.map(|s| s.small_icon[..].iter().map(|&p| p as i16).collect()),
            large_icon: smdh.map(|s| s.large_icon[..].iter().map(|&p| p as i16).collect()),

            keyword,
        }
    }

    pub fn to_ncch_info(&self) -> NcchInfo {
        fn convert_title(title: &Option<Vec<i16>>) -> Option<Vec<String>> {
            title.as_ref().map(|x| {
                let x: Vec<u16> = x.iter().map(|&y| y as u16).collect();
                let chunk = x.len() / 16;
                x.chunks(chunk).map(convert_string16).collect()
            })
        }

        let maker_code = self.maker_code as u16;
        let maker_code = [
            (maker_code & 0xFF) as u8 as char,
            (maker_code >> 8) as u8 as char,
        ]
        .iter()
        .collect();

        let service_zero_test = [0u8; 8];

        NcchInfo {
            id: self.id.clone(),
            ncch_signature: self.ncch_signature.clone(),
            content_size: self.content_size as u32,
            partition_id: format!("{:016x}", self.partition_id as u64),
            maker_code,
            ncch_version: self.ncch_verson as u16,
            program_id: format!("{:016x}", self.program_id as u64),
            product_code: convert_string(&self.product_code),
            secondary_key_slot: self.secondary_key_slot as u8,
            platform: self.platform as u8,
            content_is_data: self.content_is_data,
            content_is_executable: self.content_is_executable,
            content_category: self.content_category as u8,
            content_unit_size: self.content_unit_size as u8,
            fixed_key: self.fixed_key,
            no_romfs: self.no_romfs,
            no_crypto: self.no_crypto,
            seed_crypto: self.seed_crypto,

            exheader_name: self.exheader_name.as_ref().map(|s| convert_string(s)),
            sd_app: self.sd_app,
            remaster_version: self.remaster_version.map(|x| x as u16),
            dependencies: self.dependencies.as_ref().map(|c| {
                trim(&0, c)
                    .iter()
                    .map(|&x| format!("{:016x}", x as u64))
                    .collect()
            }),
            save_data_size: self.save_data_size.map(|x| x as u64),
            jump_id: self.jump_id.map(|x| format!("{:016x}", x as u64)),
            exheader_program_id: self
                .exheader_program_id
                .map(|x| format!("{:016x}", x as u64)),
            core_version: self.core_version.map(|x| x as u32),
            enable_l2_cache: self.enable_l2_cache,
            high_cpu_speed: self.high_cpu_speed,
            system_mode: self.system_mode.map(|x| x as u8),
            n3ds_system_mode: self.n3ds_system_mode.map(|x| x as u8),
            ideal_processor: self.ideal_processor.map(|x| x as u8),
            affinity_mask: self.affinity_mask.map(|x| x as u8),
            thread_priority: self.thread_priority.map(|x| x as u8),
            resource_limit_desc: self
                .resource_limit_desc
                .as_ref()
                .map(|x| trim(&0, x).iter().map(|&y| y as u16).collect()),
            extdata_id: self.extdata_id.map(|x| format!("{:016x}", x as u64)),
            system_savedata_id0: self
                .system_savedata_id0
                .map(|x| format!("{:08x}", x as u32)),
            system_savedata_id1: self
                .system_savedata_id1
                .map(|x| format!("{:08x}", x as u32)),
            storage_access_id: self.storage_access_id.map(|x| format!("{:016x}", x as u64)),
            filesystem_flag: self.filesystem_flag.map(|x| x as u64),
            services: self.services.as_ref().map(|x| {
                trim(&service_zero_test, x)
                    .iter()
                    .map(|y| convert_string(&y))
                    .collect()
            }),
            resource_limit_category: self.resource_limit_category.map(|x| x as u8),
            kernel_desc: self
                .kernel_desc
                .as_ref()
                .map(|x| trim(&-1, x).iter().map(|&y| y as u32).collect()),
            arm9_flag: self.arm9_flag.map(|x| x as u32),
            arm9_flag_version: self.arm9_flag_version.map(|x| x as u8),

            short_title: convert_title(&self.short_title),
            long_title: convert_title(&self.long_title),
            publisher: convert_title(&self.publisher),
            ratings: self
                .ratings
                .as_ref()
                .map(|x| x.iter().map(|&y| y as u8).collect()),
            region_lockout: self.region_lockout.map(|x| x as u32),
            match_maker_id: self.match_maker_id.map(|x| format!("{:08x}", x as u32)),
            match_maker_bit_id: self
                .match_maker_bit_id
                .map(|x| format!("{:016x}", x as u64)),
            smdh_flags: self.smdh_flags.map(|x| x as u32),
            eula_version: self.eula_version.map(|x| x as u16),
            cec_id: self.cec_id.map(|x| format!("{:08x}", x as u32)),
        }
    }
}

pub struct Database {
    pool: Pool<ConnectionManager<PgConnection>>,
}

impl std::fmt::Debug for Database {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[database]")
    }
}

impl Database {
    pub fn connect() -> Database {
        let manager =
            ConnectionManager::new(env::var("DATABASE_URL").expect("DATABASE_URL must be set"));
        let pool = Pool::builder()
            .max_size(10)
            .build(manager)
            .expect("Failed to build the connection pool");
        Database { pool }
    }

    pub fn get_connection(&self) -> Result<Connection, Box<dyn std::error::Error>> {
        Ok(self
            .pool
            .get()
            .map(|connection| Connection { connection })?)
    }
}

pub enum DatabaseError {
    Conflict,
    NotFound,
    InvalidParam,
    Other,
}

trait DatabaseTypeAdaptor {
    type DatabaseType;
    fn cast(self) -> Self::DatabaseType;
}

impl DatabaseTypeAdaptor for u8 {
    type DatabaseType = i16;
    fn cast(self) -> Self::DatabaseType {
        self as i16
    }
}

impl DatabaseTypeAdaptor for u16 {
    type DatabaseType = i16;
    fn cast(self) -> Self::DatabaseType {
        self as i16
    }
}

impl DatabaseTypeAdaptor for u32 {
    type DatabaseType = i32;
    fn cast(self) -> Self::DatabaseType {
        self as i32
    }
}

impl DatabaseTypeAdaptor for u64 {
    type DatabaseType = i64;
    fn cast(self) -> Self::DatabaseType {
        self as i64
    }
}

impl DatabaseTypeAdaptor for bool {
    type DatabaseType = bool;
    fn cast(self) -> Self::DatabaseType {
        self
    }
}

fn filter_comparator<'a, T, U>(
    statement: ncch::BoxedQuery<'a, diesel::pg::Pg>,
    comparator: &Option<Comparator>,
    rhs: &Option<StringWrapper<T>>,
    field: U,
) -> Result<ncch::BoxedQuery<'a, diesel::pg::Pg>, DatabaseError>
where
    T: DatabaseTypeAdaptor + std::string::ToString + std::str::FromStr,
    U: 'a
        + diesel::expression_methods::ExpressionMethods
        + diesel::AppearsOnTable<ncch::table>
        + diesel::expression::NonAggregate
        + diesel::query_builder::QueryFragment<diesel::pg::Pg>,
    T::DatabaseType: diesel::expression::AsExpression<U::SqlType>,
    <T::DatabaseType as diesel::expression::AsExpression<U::SqlType>>::Expression:
        'a
            + diesel::AppearsOnTable<ncch::table>
            + diesel::expression::NonAggregate
            + diesel::query_builder::QueryFragment<diesel::pg::Pg>,
{
    if let Some(cmp) = comparator {
        let rhs = rhs
            .as_ref()
            .and_then(|v| v.value())
            .ok_or(DatabaseError::InvalidParam)?
            .cast();
        Ok(match cmp {
            Comparator::Eq => statement.filter(field.eq(rhs)),
            Comparator::Ne => statement.filter(field.ne(rhs)),
            Comparator::Lt => statement.filter(field.lt(rhs)),
            Comparator::Le => statement.filter(field.le(rhs)),
            Comparator::Gt => statement.filter(field.gt(rhs)),
            Comparator::Ge => statement.filter(field.ge(rhs)),
        })
    } else {
        Ok(statement)
    }
}

fn filter_eq<'a, T, U>(
    statement: ncch::BoxedQuery<'a, diesel::pg::Pg>,
    rhs: &Option<StringWrapper<T>>,
    field: U,
) -> Result<ncch::BoxedQuery<'a, diesel::pg::Pg>, DatabaseError>
where
    T: DatabaseTypeAdaptor + std::string::ToString + std::str::FromStr,
    U: 'a
        + diesel::expression_methods::ExpressionMethods
        + diesel::AppearsOnTable<ncch::table>
        + diesel::expression::NonAggregate
        + diesel::query_builder::QueryFragment<diesel::pg::Pg>,
    T::DatabaseType: diesel::expression::AsExpression<U::SqlType>,
    <T::DatabaseType as diesel::expression::AsExpression<U::SqlType>>::Expression:
        'a
            + diesel::AppearsOnTable<ncch::table>
            + diesel::expression::NonAggregate
            + diesel::query_builder::QueryFragment<diesel::pg::Pg>,
{
    if let Some(rhs) = rhs {
        let rhs = rhs.value().ok_or(DatabaseError::InvalidParam)?.cast();
        Ok(statement.filter(field.eq(rhs)))
    } else {
        Ok(statement)
    }
}

fn filter_id<'a, U: diesel::query_source::Column>(
    statement: ncch::BoxedQuery<'a, diesel::pg::Pg>,
    id: &Option<String>,
    mask: &Option<String>,
    _: U,
) -> Result<ncch::BoxedQuery<'a, diesel::pg::Pg>, DatabaseError> {
    if let Some(id) = id {
        let id = u64::from_str_radix(&id, 16).map_err(|_| DatabaseError::InvalidParam)? as i64;
        let mask = mask.as_ref().ok_or(DatabaseError::InvalidParam)?;
        let mask = u64::from_str_radix(&mask, 16).map_err(|_| DatabaseError::InvalidParam)? as i64;
        Ok(statement.filter(diesel::dsl::sql(&format!(
            "{} IS NOT NULL AND {} & {} = {}",
            U::NAME,
            U::NAME,
            mask,
            id
        ))))
    } else {
        Ok(statement)
    }
}

fn filter_flag<'a, T, U>(
    statement: ncch::BoxedQuery<'a, diesel::pg::Pg>,
    flag: &Option<StringWrapper<T>>,
    mask: &Option<StringWrapper<T>>,
    _: U,
) -> Result<ncch::BoxedQuery<'a, diesel::pg::Pg>, DatabaseError>
where
    U: diesel::query_source::Column,
    T: DatabaseTypeAdaptor + std::string::ToString + std::str::FromStr,
    T::DatabaseType: std::fmt::Display,
{
    if let Some(flag) = flag {
        //let id = u64::from_str_radix(&id, 16).map_err(|_| DatabaseError::InvalidParam)? as i64;
        let flag = flag.value().ok_or(DatabaseError::InvalidParam)?.cast();
        let mask = mask.as_ref().ok_or(DatabaseError::InvalidParam)?;
        let mask = mask.value().ok_or(DatabaseError::InvalidParam)?.cast();
        Ok(statement.filter(diesel::dsl::sql(&format!(
            "{} IS NOT NULL AND {} & {} = {}",
            U::NAME,
            U::NAME,
            mask,
            flag
        ))))
    } else {
        Ok(statement)
    }
}

fn filter_ncch(
    param: &NcchFilterParam,
) -> Result<ncch::BoxedQuery<'_, diesel::pg::Pg>, DatabaseError> {
    let mut statement = Box::new(ncch::table).into_boxed();
    if let Some(keyword) = &param.keyword {
        let keyword_matcher = format!(
            "%{}%",
            normalize(keyword)
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
        );
        statement = statement.filter(ncch::keyword.like(keyword_matcher).escape('\\'));
    }

    statement = filter_comparator(
        statement,
        &param.content_size_cmp,
        &param.content_size_rhs,
        ncch::content_size,
    )?;

    statement = filter_id(
        statement,
        &param.partition_id,
        &param.partition_id_mask,
        ncch::partition_id,
    )?;

    if let Some(maker_code) = &param.maker_code {
        if maker_code.len() != 2 {
            return Err(DatabaseError::InvalidParam);
        }
        let maker_code =
            (maker_code.as_bytes()[0] as u16 + ((maker_code.as_bytes()[1] as u16) << 8)) as i16;
        statement = statement.filter(ncch::maker_code.eq(maker_code));
    }

    statement = filter_comparator(
        statement,
        &param.ncch_version_cmp,
        &param.ncch_version_rhs,
        ncch::ncch_verson,
    )?;

    statement = filter_id(
        statement,
        &param.program_id,
        &param.program_id_mask,
        ncch::program_id,
    )?;

    if let Some(product_code) = &param.product_code {
        if product_code.len() > 16 {
            return Err(DatabaseError::InvalidParam);
        }
        let mut product_code = product_code.as_bytes().to_vec();
        while product_code.len() < 16 {
            product_code.push(0);
        }
        statement = statement.filter(ncch::product_code.eq(product_code));
    }

    statement = filter_comparator(
        statement,
        &param.secondary_key_slot_cmp,
        &param.secondary_key_slot_rhs,
        ncch::secondary_key_slot,
    )?;

    statement = filter_eq(statement, &param.platform, ncch::platform)?;

    statement = filter_eq(statement, &param.content_is_data, ncch::content_is_data)?;

    statement = filter_eq(
        statement,
        &param.content_is_executable,
        ncch::content_is_executable,
    )?;

    statement = filter_comparator(
        statement,
        &param.content_category_cmp,
        &param.content_category_rhs,
        ncch::content_category,
    )?;

    statement = filter_comparator(
        statement,
        &param.content_unit_size_cmp,
        &param.content_unit_size_rhs,
        ncch::content_unit_size,
    )?;

    statement = filter_eq(statement, &param.fixed_key, ncch::fixed_key)?;

    statement = filter_eq(statement, &param.no_romfs, ncch::no_romfs)?;

    statement = filter_eq(statement, &param.no_crypto, ncch::no_crypto)?;

    statement = filter_eq(statement, &param.seed_crypto, ncch::seed_crypto)?;

    if let Some(exheader_name) = &param.exheader_name {
        if exheader_name.len() > 8 {
            return Err(DatabaseError::InvalidParam);
        }
        let mut exheader_name = exheader_name.as_bytes().to_vec();
        while exheader_name.len() < 8 {
            exheader_name.push(0);
        }
        statement = statement.filter(ncch::exheader_name.eq(exheader_name));
    }

    statement = filter_eq(statement, &param.sd_app, ncch::sd_app)?;

    statement = filter_comparator(
        statement,
        &param.save_data_size_cmp,
        &param.save_data_size_rhs,
        ncch::save_data_size,
    )?;

    statement = filter_id(
        statement,
        &param.jump_id,
        &param.jump_id_mask,
        ncch::jump_id,
    )?;

    statement = filter_id(
        statement,
        &param.exheader_program_id,
        &param.exheader_program_id_mask,
        ncch::exheader_program_id,
    )?;

    statement = filter_comparator(
        statement,
        &param.core_version_cmp,
        &param.core_version_rhs,
        ncch::core_version,
    )?;

    statement = filter_eq(statement, &param.enable_l2_cache, ncch::enable_l2_cache)?;

    statement = filter_eq(statement, &param.high_cpu_speed, ncch::high_cpu_speed)?;

    statement = filter_comparator(
        statement,
        &param.system_mode_cmp,
        &param.system_mode_rhs,
        ncch::system_mode,
    )?;

    statement = filter_comparator(
        statement,
        &param.n3ds_system_mode_cmp,
        &param.n3ds_system_mode_rhs,
        ncch::n3ds_system_mode,
    )?;

    statement = filter_comparator(
        statement,
        &param.ideal_processor_cmp,
        &param.ideal_processor_rhs,
        ncch::ideal_processor,
    )?;

    statement = filter_comparator(
        statement,
        &param.affinity_mask_cmp,
        &param.affinity_mask_rhs,
        ncch::affinity_mask,
    )?;

    statement = filter_comparator(
        statement,
        &param.thread_priority_cmp,
        &param.thread_priority_rhs,
        ncch::thread_priority,
    )?;

    statement = filter_flag(
        statement,
        &param.filesystem_flag,
        &param.filesystem_flag_mask,
        ncch::filesystem_flag,
    )?;

    statement = filter_comparator(
        statement,
        &param.resource_limit_category_cmp,
        &param.resource_limit_category_rhs,
        ncch::resource_limit_category,
    )?;

    statement = filter_flag(
        statement,
        &param.arm9_flag,
        &param.arm9_flag_mask,
        ncch::arm9_flag,
    )?;

    statement = filter_comparator(
        statement,
        &param.arm9_flag_version_cmp,
        &param.arm9_flag_version_rhs,
        ncch::arm9_flag_version,
    )?;

    statement = filter_flag(
        statement,
        &param.region_lockout,
        &param.region_lockout_mask,
        ncch::region_lockout,
    )?;

    if let Some(match_maker_id) = &param.match_maker_id {
        let match_maker_id = u32::from_str_radix(&match_maker_id, 16)
            .map_err(|_| DatabaseError::InvalidParam)? as i32;
        statement = statement.filter(ncch::match_maker_id.eq(match_maker_id));
    }

    if let Some(match_maker_bit_id) = &param.match_maker_bit_id {
        let match_maker_bit_id = u64::from_str_radix(&match_maker_bit_id, 16)
            .map_err(|_| DatabaseError::InvalidParam)? as i64;
        statement = statement.filter(ncch::match_maker_bit_id.eq(match_maker_bit_id));
    }

    statement = filter_flag(
        statement,
        &param.smdh_flag,
        &param.smdh_flag_mask,
        ncch::smdh_flags,
    )?;

    statement = filter_comparator(
        statement,
        &param.eula_version_cmp,
        &param.eula_version_rhs,
        ncch::eula_version,
    )?;

    if let Some(cec_id) = &param.cec_id {
        let cec_id =
            u32::from_str_radix(&cec_id, 16).map_err(|_| DatabaseError::InvalidParam)? as i32;
        statement = statement.filter(ncch::cec_id.eq(cec_id));
    }

    Ok(statement)
}

pub struct Connection {
    connection: PooledConnection<ConnectionManager<PgConnection>>,
}

impl Connection {
    pub fn insert_ncch_record(&self, record: &NcchRecord) -> Result<(), DatabaseError> {
        let record: QueryResult<NcchRecord> = diesel::insert_into(ncch::table)
            .values(record)
            .get_result(&self.connection);
        match record {
            Ok(_) => {
                info!("NCCH record inserted");
                Ok(())
            }
            Err(Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => {
                warn!("NCCH record already exits");
                Err(DatabaseError::Conflict)
            }
            Err(e) => {
                error!("Database error: {}", e);
                Err(DatabaseError::Other)
            }
        }
    }

    pub fn get_ncch_record(&self, id: &str) -> Result<NcchRecord, DatabaseError> {
        info!("getting NCCH with id = {}", id);
        match ncch::table.filter(ncch::id.eq(id)).first(&self.connection) {
            Err(e) => {
                warn!("Database error: {}", e);
                Err(DatabaseError::NotFound)
            }
            Ok(ncch) => {
                info!("NCCH found");
                Ok(ncch)
            }
        }
    }

    pub fn query_ncch(&self, param: &NcchQueryParam) -> Result<Vec<NcchRecord>, DatabaseError> {
        if param.limit < 1 || param.limit > 100 || param.offset < 0 {
            return Err(DatabaseError::InvalidParam);
        }
        match filter_ncch(&param.filter)?
            .order_by(ncch::program_id.asc())
            .then_order_by(ncch::id.asc())
            .limit(param.limit)
            .offset(param.offset)
            .load(&self.connection)
        {
            Err(e) => {
                error!("Database error: {}", e);
                Err(DatabaseError::Other)
            }
            Ok(ncchs) => Ok(ncchs),
        }
    }

    pub fn query_ncch_count(&self, param: &NcchFilterParam) -> Result<i64, DatabaseError> {
        match filter_ncch(param)?
            .select(diesel::dsl::count(ncch::id))
            .first(&self.connection)
        {
            Err(e) => {
                error!("Database error: {}", e);
                Err(DatabaseError::Other)
            }
            Ok(count) => Ok(count),
        }
    }
}
