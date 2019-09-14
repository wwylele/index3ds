#![recursion_limit = "128"]

mod aes;
mod api;
mod data_format;
mod database;
mod key;
mod rsa2048;
mod schema;

#[macro_use]
extern crate diesel;

use actix_web::{web, App, HttpResponse, HttpServer};
use aes::*;
use api::*;
use byte_struct::*;
use data_format::*;
use database::{Database, DatabaseError};
use dotenv::dotenv;
use lazy_static::*;
use log::{error, info, warn};
use rand::prelude::*;
use rsa2048::*;
use rustls::*;
use sha2::*;
use std::collections::HashMap;
use std::io::Read;
use std::mem::drop;
use std::sync::{Arc, Mutex, RwLock};
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
enum PostNcchSessionState {
    HeaderNeeeded,
    ExheaderNeeded(NcchHeader, [u8; 16], [u8; 16], [u8; 16]),
    ExefsNeeded(NcchHeader, Option<Exheader>, [u8; 16], [u8; 16]),
    IconNeeded(
        NcchHeader,
        Option<Exheader>,
        [u8; 32],
        Option<([u8; 16], [u8; 16], u64)>,
    ),
    Finished,
    Undefined,
}

#[derive(Debug)]
struct PostNcchSession {
    id: u32,
    database: Arc<Database>,
    last_touch: Instant,
    state: PostNcchSessionState,
}

fn respond_with_icon(icon: &Option<Vec<i16>>) -> HttpResponse {
    if let Some(icon) = icon.as_ref() {
        let width = match icon.len() {
            576 => 24,
            2304 => 48,
            _ => {
                error!("unexpected icon size {}", icon.len());
                return HttpResponse::InternalServerError().finish();
            }
        };

        let block_count = width / 8;

        let mut buffer = Vec::with_capacity(width * width * 3);
        let xlut = [0x00, 0x01, 0x04, 0x05, 0x10, 0x11, 0x14, 0x15];
        let ylut = [0x00, 0x02, 0x08, 0x0a, 0x20, 0x22, 0x28, 0x2a];
        fn convert5(v: u16) -> u8 {
            ((v << 3) | (v >> 2)) as u8
        }
        fn convert6(v: u16) -> u8 {
            ((v << 2) | (v >> 4)) as u8
        }

        for y in 0..width {
            for x in 0..width {
                let bx = x / 8;
                let by = y / 8;
                let cx = x % 8;
                let cy = y % 8;
                let i = xlut[cx] + ylut[cy] + (bx + by * block_count) * 64;
                let pixel = icon[i] as u16;

                let r = convert5(pixel >> 11);
                let g = convert6((pixel >> 5) & 0b11_1111);
                let b = convert5(pixel & 0b11111);
                buffer.push(r);
                buffer.push(g);
                buffer.push(b);
            }
        }

        let mut encoded = std::io::Cursor::new(Vec::new());
        let mut encoder = png::Encoder::new(&mut encoded, width as u32, width as u32);
        encoder.set_color(png::ColorType::RGB);
        encoder.set_depth(png::BitDepth::Eight);
        match encoder.write_header() {
            Err(e) => {
                error!("PNG write_header error: {}", e);
                return HttpResponse::InternalServerError().finish();
            }
            Ok(mut writer) => match writer.write_image_data(&buffer) {
                Err(e) => {
                    error!("PNG write_image_data error: {}", e);
                    return HttpResponse::InternalServerError().finish();
                }
                Ok(()) => (),
            },
        }
        let body = actix_web::web::Bytes::from(encoded.into_inner());

        HttpResponse::Ok().content_type("image/png").body(body)
    } else {
        HttpResponse::NotFound().finish()
    }
}

impl PostNcchSession {
    pub fn new(id: u32, database: Arc<Database>) -> PostNcchSession {
        PostNcchSession {
            id,
            database,
            last_touch: Instant::now(),
            state: PostNcchSessionState::HeaderNeeeded,
        }
    }

    fn verify_and_fix_header(header: &mut NcchHeader, public_key: &[u8]) -> bool {
        info!("verifying NCCH header");
        let mut raw = [0; NcchHeader::BYTE_LEN];
        header.write_bytes(&mut raw);
        if verify_signature(&raw[0x100..], &header.signature, public_key) {
            return true;
        }

        info!("verifying NCCH first try failed. Trying with modified encryption flag");
        info!(
            "Original flag: secondary_key_slot = {}, {:?}",
            header.secondary_key_slot, header.key_config
        );
        header.key_config.no_crypto = 0;
        header.key_config.fixed_key = 0;
        for &secondary_key_slot in &[0, 1, 10, 11] {
            for &seed_crypto in &[0, 1] {
                info!(
                    "attempting secondary_key_slot = {} seed_crypto = {}",
                    secondary_key_slot, seed_crypto
                );
                header.secondary_key_slot = secondary_key_slot;
                header.key_config.seed_crypto = seed_crypto;
                header.write_bytes(&mut raw);
                if verify_signature(&raw[0x100..], &header.signature, public_key) {
                    return true;
                }
            }
        }

        false
    }

    fn request_exheader(
        &mut self,
        mut header: NcchHeader,
        key: [u8; 16],
        ctr_exheader: [u8; 16],
        ctr_exefs: [u8; 16],
    ) -> HttpResponse {
        if header.exheader_size != 0 {
            info!("requesting exheader");
            if header.exheader_size != 0x400 {
                warn!("unexpected exheader size {}", header.exheader_size);
                self.state = PostNcchSessionState::Finished;
                return PostNcchResponse::UnexpectedFormat.http();
            }
            self.state = PostNcchSessionState::ExheaderNeeded(header, key, ctr_exheader, ctr_exefs);
            PostNcchResponse::AppendNeeded(AppendRequest {
                session_id: self.id,
                offset: 0x200,
                len: 0x800,
            })
            .http()
        } else {
            info!("skipping exheader, verifying signature as CFA");

            if !PostNcchSession::verify_and_fix_header(&mut header, &*key::CFA_PUBLIC_KEY) {
                warn!("NCCH header verification failed");
                self.state = PostNcchSessionState::Finished;
                return PostNcchResponse::VerificationFailed.http();
            }

            self.request_exefs(header, None, key, ctr_exefs)
        }
    }

    fn request_exefs(
        &mut self,
        header: NcchHeader,
        exheader: Option<Exheader>,
        key: [u8; 16],
        ctr_exefs: [u8; 16],
    ) -> HttpResponse {
        if header.exefs_offset != 0 {
            info!("requesting exefs");
            let unit_size = header.unit_size();
            let exefs_offset = header.exefs_offset as usize * unit_size;
            let exefs_needed_len = std::cmp::max(
                header.exefs_hash_region_size as usize * unit_size,
                ExefsHeader::BYTE_LEN,
            );

            self.state = PostNcchSessionState::ExefsNeeded(header, exheader, key, ctr_exefs);
            PostNcchResponse::AppendNeeded(AppendRequest {
                session_id: self.id,
                offset: exefs_offset,
                len: exefs_needed_len,
            })
            .http()
        } else {
            info!("skipping exefs");
            self.finalize(header, exheader, None)
        }
    }

    fn request_icon(
        &mut self,
        header: NcchHeader,
        exheader: Option<Exheader>,
        exefs: ExefsHeader,
        exefs_crypto: Option<([u8; 16], [u8; 16])>,
    ) -> HttpResponse {
        if let Some((icon_index, icon_file)) = exefs
            .files
            .iter()
            .enumerate()
            .find(|(_, f)| f.name == *b"icon\0\0\0\0")
        {
            info!("requesting icon");
            let unit_size = header.unit_size();
            let exefs_offset = header.exefs_offset as usize * unit_size;

            let icon_offset = ExefsHeader::BYTE_LEN + icon_file.offset as usize;
            let icon_len = icon_file.size as usize;
            if icon_len != Smdh::BYTE_LEN {
                warn!("unexpected icon size {}. Skipping icon", icon_len);
                if icon_len != 0 {
                    error!("Really strange icon here");
                }
                return self.finalize(header, exheader, None);
            }
            let hash = exefs.hashes[9 - icon_index];
            self.state = PostNcchSessionState::IconNeeded(
                header,
                exheader,
                hash,
                exefs_crypto.map(|(key, ctr)| (key, ctr, icon_offset as u64)),
            );
            PostNcchResponse::AppendNeeded(AppendRequest {
                session_id: self.id,
                offset: exefs_offset + icon_offset,
                len: icon_len,
            })
            .http()
        } else {
            info!("skipping icon");
            self.finalize(header, exheader, None)
        }
    }

    fn finalize(
        &mut self,
        header: NcchHeader,
        exheader: Option<Exheader>,
        icon: Option<Smdh>,
    ) -> HttpResponse {
        info!("finalizing NCCH post");

        let connection = match self.database.get_connection() {
            Ok(connection) => connection,
            Err(e) => {
                error!("failed to get database connection: {}", e);
                self.state = PostNcchSessionState::Finished;
                return PostNcchResponse::InternalServerError.http();
            }
        };

        let record = database::NcchRecord::new(header, exheader, icon);

        match connection.insert_ncch_record(&record) {
            Ok(()) => (),
            Err(DatabaseError::Conflict) => {
                self.state = PostNcchSessionState::Finished;
                return PostNcchResponse::Conflict(NcchExist { ncch_id: record.id }).http();
            }
            Err(_) => {
                self.state = PostNcchSessionState::Finished;
                return PostNcchResponse::InternalServerError.http();
            }
        }

        self.state = PostNcchSessionState::Finished;
        PostNcchResponse::Finished(NcchExist { ncch_id: record.id }).http()
    }

    fn receive_header(&mut self, data: web::Bytes) -> HttpResponse {
        info!("reading NCCH header");
        if data.len() != NcchHeader::BYTE_LEN {
            warn!("unexpected NCCH header len: {}", data.len());
            self.state = PostNcchSessionState::Finished;
            return PostNcchResponse::UnexpectedLength.http();
        }

        let header = NcchHeader::read_bytes(&data);

        if header.magic != *b"NCCH" {
            warn!("unexpected NCCH magic: {:?}", header.magic);
            self.state = PostNcchSessionState::Finished;
            return PostNcchResponse::UnexpectedFormat.http();
        }

        let key = if header.key_config.fixed_key != 0 {
            [0; 16]
        } else {
            let mut key_y = [0; 16];
            key_y[..].copy_from_slice(&header.signature[0..0x10]);
            get_ncch_key(&key_y)
        };

        let mut ctr_exheader = [0; 16];
        let mut ctr_exefs;
        info!("NCCH version = {}", header.version);
        if header.version == 0 || header.version == 2 {
            ctr_exheader[0..8].copy_from_slice(&header.partition_id.to_be_bytes());
            ctr_exefs = ctr_exheader;
            ctr_exheader[8] = 1;
            ctr_exefs[8] = 2;
        } else if header.version == 1 {
            ctr_exheader[0..8].copy_from_slice(&header.partition_id.to_le_bytes());
            ctr_exefs = ctr_exheader;
            ctr_exheader[12..16].copy_from_slice(&0x200u32.to_be_bytes());
            ctr_exefs[12..16]
                .copy_from_slice(&(header.exefs_size * (header.unit_size() as u32)).to_be_bytes())
        } else {
            error!("Unknown NCCH version!");
            ctr_exefs = [0; 16];
        }

        if header.unit_size() != 0x200 {
            error!("weird unit size: {}", header.unit_size());
        }

        self.request_exheader(header, key, ctr_exheader, ctr_exefs)
    }

    fn receive_exheader(
        &mut self,
        data: web::Bytes,
        mut header: NcchHeader,
        key: [u8; 16],
        ctr_exheader: [u8; 16],
        ctr_exefs: [u8; 16],
    ) -> HttpResponse {
        info!("reading Exheader");
        if data.len() != Exheader::BYTE_LEN {
            warn!("unexpected Exheader header len: {}", data.len());
            self.state = PostNcchSessionState::Finished;
            return PostNcchResponse::UnexpectedLength.http();
        }

        let mut data = &data[..];
        let mut temp;
        let mut hasher = Sha256::new();
        hasher.input(&data[0..0x400]);
        if hasher.result()[..] != header.exheader_hash[..] {
            info!("decrypting exheader");
            temp = data[..].to_vec();
            aes_ctr_decrypt(&mut temp, &key, &ctr_exheader, 0);
            data = &temp[..];

            let mut hasher = Sha256::new();
            hasher.input(&data[0..0x400]);
            if hasher.result()[..] != header.exheader_hash[..] {
                warn!("Exheader hash mismatch");
                self.state = PostNcchSessionState::Finished;
                return PostNcchResponse::VerificationFailed.http();
            }
        }

        let exheader = Exheader::read_bytes(&data);

        if !verify_signature(
            &data[0x500..],
            &exheader.signature,
            &*key::EXHEADER_PUBLIC_KEY,
        ) {
            warn!("Exheader verification failed");
            self.state = PostNcchSessionState::Finished;
            return PostNcchResponse::VerificationFailed.http();
        }

        if !PostNcchSession::verify_and_fix_header(&mut header, &exheader.public_key) {
            warn!("NCCH header verification failed");
            self.state = PostNcchSessionState::Finished;
            return PostNcchResponse::VerificationFailed.http();
        }

        self.request_exefs(header, Some(exheader), key, ctr_exefs)
    }

    fn receive_exefs(
        &mut self,
        data: web::Bytes,
        header: NcchHeader,
        exheader: Option<Exheader>,
        key: [u8; 16],

        ctr_exefs: [u8; 16],
    ) -> HttpResponse {
        info!("reading Exefs");

        let unit_size = header.unit_size();
        let exefs_hash_region_size = header.exefs_hash_region_size as usize * unit_size;

        if data.len() != std::cmp::max(ExefsHeader::BYTE_LEN, exefs_hash_region_size) {
            warn!("unexpected Exefs len: {}", data.len());
            self.state = PostNcchSessionState::Finished;
            return PostNcchResponse::UnexpectedLength.http();
        }

        let mut icon_crypto = None;

        let mut data = &data[..];
        let mut temp;
        let mut hasher = Sha256::new();
        hasher.input(&data[0..exefs_hash_region_size]);
        if hasher.result()[..] != header.exefs_hash[..] {
            info!("decrypting exefs");
            temp = data.to_vec();
            aes_ctr_decrypt(&mut temp, &key, &ctr_exefs, 0);
            data = &temp[..];
            icon_crypto = Some((key, ctr_exefs));

            let mut hasher = Sha256::new();
            hasher.input(&data[0..exefs_hash_region_size]);
            if hasher.result()[..] != header.exefs_hash[..] {
                warn!("Exefs hash mismatch");
                self.state = PostNcchSessionState::Finished;
                return PostNcchResponse::VerificationFailed.http();
            }
        }

        let exefs = ExefsHeader::read_bytes(&data[0..ExefsHeader::BYTE_LEN]);

        self.request_icon(header, exheader, exefs, icon_crypto)
    }

    fn receive_icon(
        &mut self,
        data: web::Bytes,
        header: NcchHeader,
        exheader: Option<Exheader>,
        hash: [u8; 32],
        icon_crypto: Option<([u8; 16], [u8; 16], u64)>,
    ) -> HttpResponse {
        info!("reading icon");

        if data.len() != Smdh::BYTE_LEN {
            warn!("unexpected icon len: {}", data.len());
            self.state = PostNcchSessionState::Finished;
            return PostNcchResponse::UnexpectedLength.http();
        }

        let mut data = &data[..];
        let mut temp;
        if let Some((key, ctr, offset)) = icon_crypto {
            info!("decrypting icon");
            temp = data.to_vec();
            aes_ctr_decrypt(&mut temp, &key, &ctr, offset);
            data = &temp[..];
        }

        let mut hasher = Sha256::new();
        hasher.input(&data);
        if hasher.result()[..] != hash {
            warn!("icon hash mismatch");
            self.state = PostNcchSessionState::Finished;
            return PostNcchResponse::VerificationFailed.http();
        }

        let smdh = Smdh::read_bytes(&data);
        if smdh.magic != *b"SMDH" {
            error!("unexpected SMDH magic: {:?}", smdh.magic);
            return self.finalize(header, exheader, None);
        }

        self.finalize(header, exheader, Some(smdh))
    }

    pub fn next(&mut self, data: web::Bytes) -> HttpResponse {
        self.last_touch = Instant::now();
        match std::mem::replace(&mut self.state, PostNcchSessionState::Undefined) {
            PostNcchSessionState::HeaderNeeeded => self.receive_header(data),

            PostNcchSessionState::ExheaderNeeded(header, key, ctr_exheader, ctr_exefs) => {
                self.receive_exheader(data, header, key, ctr_exheader, ctr_exefs)
            }

            PostNcchSessionState::ExefsNeeded(header, exheader, key, ctr_exefs) => {
                self.receive_exefs(data, header, exheader, key, ctr_exefs)
            }

            PostNcchSessionState::IconNeeded(header, exheader, hash, icon_crypto) => {
                self.receive_icon(data, header, exheader, hash, icon_crypto)
            }

            PostNcchSessionState::Finished => {
                warn!("already finished session");
                PostNcchResponse::AlreadyFinished.http()
            }

            PostNcchSessionState::Undefined => {
                error!("The session is in the undefined state");
                PostNcchResponse::AlreadyFinished.http()
            }
        }
    }

    pub fn finished(&self) -> bool {
        match self.state {
            PostNcchSessionState::Finished | PostNcchSessionState::Undefined => true,
            _ => false,
        }
    }

    pub fn last_touch(&self) -> Instant {
        self.last_touch
    }
}

lazy_static! {
    pub static ref STATIC_ROOT: String = std::env::var("STATIC_ROOT").expect("STATIC_ROOT");
}

fn static_file(path: &str) -> actix_web::Route {
    let path = format!("{}{}", &*STATIC_ROOT, path);
    web::get().to(move || actix_files::NamedFile::open(&path).expect("Unable to open file"))
}

fn main() -> std::io::Result<()> {
    println!(" === Index3DS === ");
    stderrlog::new()
        .module(module_path!())
        .verbosity(2)
        .init()
        .unwrap();
    info!("Log init");

    dotenv().ok();

    let database_root = Arc::new(Database::connect());

    info!("Database connected");

    let session_cleanup_period = Duration::from_secs(
        std::env::var("SESSION_CLEANUP_PERIOD")
            .expect("SESSION_CLEANUP_PERIOD")
            .parse()
            .unwrap(),
    );
    let max_session_count = std::env::var("MAX_SESSION_COUNT")
        .expect("MAX_SESSION_COUNT")
        .parse()
        .unwrap();

    let session_map_root = Arc::new(RwLock::new(
        HashMap::<u32, Arc<Mutex<PostNcchSession>>>::new(),
    ));

    let session_map = session_map_root.clone();
    let cleanup_session_root = move || {
        let mut session_map = session_map.write().unwrap();
        session_map.retain(|_, session| {
            if let Ok(session) = session.try_lock() {
                (!session.finished() && session.last_touch().elapsed() < session_cleanup_period)
            } else {
                true
            }
        })
    };

    let cleanup_session = cleanup_session_root.clone();
    spawn(move || loop {
        sleep(session_cleanup_period);
        cleanup_session();
    });

    let mut server = HttpServer::new(move || {
        let session_map = session_map_root.clone();
        let database = database_root.clone();

        let cleanup_session = cleanup_session_root.clone();
        let post_ncch = move |ncch_header: web::Bytes| {
            info!("post_ncch called");
            info!("ncch_header.len = {}", ncch_header.len());
            let mut session_map = {
                let session_map_write = session_map.write().unwrap();
                if session_map_write.len() <= max_session_count {
                    session_map_write
                } else {
                    warn!("Session capacity reached! Try cleaning up");
                    drop(session_map_write);
                    cleanup_session();
                    let session_map_write = session_map.write().unwrap();
                    if session_map_write.len() <= max_session_count {
                        session_map_write
                    } else {
                        error!("Session capacity reached and could not reduce!");
                        return PostNcchResponse::Busy.http();
                    }
                }
            };

            let session_id = loop {
                let session_id = random();
                if !session_map.contains_key(&session_id) {
                    break session_id;
                }
            };

            let session = Arc::new(Mutex::new(PostNcchSession::new(
                session_id,
                database.clone(),
            )));
            session_map.insert(session_id, session.clone());
            drop(session_map);

            let mut session = session.lock().unwrap();
            session.next(ncch_header)
        };

        let session_map = session_map_root.clone();
        let append_ncch = move |path: web::Path<(u32,)>, data: web::Bytes| {
            info!("append_ncch called");
            let session_id = path.0;
            info!("session_id = {}, data.len = {}", session_id, data.len());
            let session_map = session_map.read().unwrap();
            let session = session_map.get(&session_id).cloned();
            drop(session_map);
            if let Some(session) = session {
                let mut session = session.lock().unwrap();
                session.next(data)
            } else {
                PostNcchResponse::NotFound.http()
            }
        };

        let database = database_root.clone();
        let ncch_info = move |path: web::Path<(String, String)>| {
            info!("ncch_info called");
            let ncch_id = &path.0;
            let info_type: &str = &path.1;
            info!("ncch_id = {}, info_type = {}", ncch_id, info_type);
            let connection = match database.get_connection() {
                Ok(connection) => connection,
                Err(e) => {
                    error!("failed to get database connection: {}", e);
                    return NcchInfoResponse::InternalServerError.http();
                }
            };
            let record = match connection.get_ncch_record(ncch_id) {
                Ok(record) => record,
                Err(DatabaseError::NotFound) => {
                    warn!("NCCH record not found");
                    return NcchInfoResponse::NotFound.http();
                }
                Err(_) => {
                    error!("unhandled error when getting NCCH record");
                    return NcchInfoResponse::InternalServerError.http();
                }
            };

            match info_type {
                "info" => NcchInfoResponse::Ok(record.to_ncch_info()).http(),
                "icon_small.png" => respond_with_icon(&record.small_icon),
                "icon_large.png" => respond_with_icon(&record.large_icon),
                _ => NcchInfoResponse::NotFound.http(),
            }
        };

        let database = database_root.clone();
        let query_ncch = move |param: web::Query<NcchQueryParam>| {
            info!("NCCH query called");
            let connection = match database.get_connection() {
                Ok(connection) => connection,
                Err(e) => {
                    error!("failed to get database connection: {}", e);
                    return NcchQueryResponse::InternalServerError.http();
                }
            };

            match connection.query_ncch(&param) {
                Ok(records) => NcchQueryResponse::Ok(NcchInfoVec {
                    ncchs: records
                        .iter()
                        .map(database::NcchRecord::to_ncch_info)
                        .collect(),
                })
                .http(),
                Err(_) => {
                    error!("unhandled error when getting NCCH record");
                    NcchQueryResponse::InternalServerError.http()
                }
            }
        };

        let database = database_root.clone();
        let query_ncch_count = move |param: web::Query<NcchFilterParam>| {
            info!("NCCH query count called");
            let connection = match database.get_connection() {
                Ok(connection) => connection,
                Err(e) => {
                    error!("failed to get database connection: {}", e);
                    return NcchQueryResponse::InternalServerError.http();
                }
            };

            match connection.query_ncch_count(&param) {
                Ok(count) => NcchQueryCountResponse::Ok(NcchCount { count }).http(),
                Err(_) => {
                    error!("unhandled error when getting NCCH record");
                    NcchQueryResponse::InternalServerError.http()
                }
            }
        };

        let index = || static_file("index.html");

        App::new()
            .route(url::post_ncch(), web::post().to(post_ncch))
            .route(
                &url::append_ncch("{session_id}"),
                web::post().to(append_ncch),
            )
            .route(
                &url::ncch_info("{ncch_id}", "{info_type}"),
                web::get().to(ncch_info),
            )
            .route(url::query_ncch(), web::get().to(query_ncch))
            .route(url::query_ncch_count(), web::get().to(query_ncch_count))
            .route(url::ncch(), index())
            .route(url::submit_ncch(), index())
            .route(url::ncch_list(), index())
            .service(actix_files::Files::new("/", &*STATIC_ROOT))
    });

    let addr = std::env::var("BIND_POINT").expect("BIND_POINT");

    if std::env::var("HTTPS").ok().as_ref().map(|x| x.as_str()) == Some("1") {
        let mut cert = vec![];
        let mut key = vec![];
        std::fs::File::open(std::env::var("CERTIFICATE").expect("CERTIFICATE"))?
            .read_to_end(&mut cert)?;
        std::fs::File::open(std::env::var("PRIVATE_KEY").expect("PRIVATE_KEY"))?
            .read_to_end(&mut key)?;
        let cert = rustls::internal::pemfile::certs(&mut &cert[..]).unwrap();
        let key = rustls::internal::pemfile::pkcs8_private_keys(&mut &key[..]).unwrap().remove(0);
        let mut config = ServerConfig::new(NoClientAuth::new());
        config.set_single_cert(cert, key).unwrap();
        server = server.bind_rustls(addr, config)?;
    } else {
        server = server.bind(addr)?;
    }

    server.run()
}
