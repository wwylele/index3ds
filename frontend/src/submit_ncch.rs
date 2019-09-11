use index3ds_common::*;
use std::cell::*;
use std::rc::Rc;
use stdweb::web::event::LoadEndEvent;
use stdweb::web::{FileReader, FileReaderResult, IEventTarget, TypedArray};
use yew::callback::Callback;
use yew::format::json::Json;
use yew::services::fetch::*;
use yew::services::reader::{File, IBlob};
use yew::{html, ChangeData, Component, ComponentLink, Html, Renderable, ShouldRender};

#[derive(Debug)]
enum SubmitStatus {
    Submitting,
    Succeeded(String),
    Conflict(String),
    Busy,
    VerificationFailed,
    ServerError,
    FormatError,
}

pub struct SubmitEntry {
    name: String,
    status: SubmitStatus,
}

struct ReaderTaskEx {
    file_reader: FileReader,
}

fn read_file_ex<T: IBlob>(file: T, callback: Callback<Vec<u8>>) -> ReaderTaskEx {
    let file_reader = FileReader::new();
    let reader = file_reader.clone();
    file_reader.add_event_listener(move |_event: LoadEndEvent| match reader.result() {
        Some(FileReaderResult::String(_)) => {
            unreachable!();
        }
        Some(FileReaderResult::ArrayBuffer(buffer)) => {
            let array: TypedArray<u8> = buffer.into();
            callback.emit(array.to_vec());
        }
        None => {}
    });
    file_reader.read_as_array_buffer(&file).unwrap();
    ReaderTaskEx { file_reader }
}

pub struct PageSubmitNcch {
    link: ComponentLink<PageSubmitNcch>,
    submits: Vec<Rc<RefCell<SubmitEntry>>>,
    reader_task: Vec<ReaderTaskEx>,
    fetch_service: FetchService,
    fetch_task: Vec<FetchTask>,
}

pub enum Msg {
    Files(Vec<File>),
    AddFailedSubmit(String),
    StartProcessNcch(File, u64),
    StartProcessNcsd(File),
    SendNcchFirst(Rc<RefCell<SubmitEntry>>, File, u64, Vec<u8>),
    ProcessMoreNcch(Rc<RefCell<SubmitEntry>>, File, u64, u32, usize, usize),
    SendMoreNcch(Rc<RefCell<SubmitEntry>>, File, u64, Vec<u8>, u32),
    NcsdToNcch(File, Vec<u8>),
    None,
}

fn process_response<T: std::fmt::Display>(
    body: Result<PostNcchResponse, T>,
    entry: Rc<RefCell<SubmitEntry>>,
    file: File,
    base_offset: u64,
) -> Msg {
    match body {
        Ok(PostNcchResponse::Finished(NcchExist { ncch_id })) => {
            entry.borrow_mut().status = SubmitStatus::Succeeded(ncch_id);
            Msg::None
        }
        Ok(PostNcchResponse::AppendNeeded(AppendRequest {
            session_id,
            offset,
            len,
        })) => Msg::ProcessMoreNcch(entry, file, base_offset, session_id, offset, len),
        Ok(PostNcchResponse::Conflict(NcchExist { ncch_id })) => {
            entry.borrow_mut().status = SubmitStatus::Conflict(ncch_id);
            Msg::None
        }
        Ok(PostNcchResponse::Busy) => {
            entry.borrow_mut().status = SubmitStatus::Busy;
            Msg::None
        }
        Ok(PostNcchResponse::VerificationFailed) => {
            entry.borrow_mut().status = SubmitStatus::VerificationFailed;
            Msg::None
        }
        Ok(PostNcchResponse::UnexpectedFormat) => {
            entry.borrow_mut().status = SubmitStatus::FormatError;
            Msg::None
        }
        _ => {
            entry.borrow_mut().status = SubmitStatus::ServerError;
            Msg::None
        }
    }
}

const PARTITION_NAMES: [&str; 8] = [
    "Executable",
    "E-Manual",
    "Download Play child",
    "Partition 3",
    "Partition 4",
    "Partition 5",
    "N3DS system update",
    "System update",
];

impl PageSubmitNcch {
    fn process_ncch(&mut self, label_extra: &str, file: File, offset: u64) {
        let entry = Rc::new(RefCell::new(SubmitEntry {
            name: format!("{} ({})", file.name(), label_extra),
            status: SubmitStatus::Submitting,
        }));
        self.submits.push(entry.clone());
        self.reader_task.push(read_file_ex(
            file.slice(offset..offset + 0x200),
            self.link.send_back(move |data: Vec<u8>| {
                Msg::SendNcchFirst(entry.clone(), file.clone(), offset, data)
            }),
        ));
    }

    fn process_more_ncch(
        &mut self,
        entry: Rc<RefCell<SubmitEntry>>,
        file: File,
        base_offset: u64,
        session_id: u32,
        offset: usize,
        len: usize,
    ) {
        let begin = base_offset + offset as u64;
        let end = begin + len as u64;
        self.reader_task.push(read_file_ex(
            file.slice(begin..end),
            self.link.send_back(move |data: Vec<u8>| {
                Msg::SendMoreNcch(entry.clone(), file.clone(), base_offset, data, session_id)
            }),
        ));
    }
    fn send_more_ncch(
        &mut self,
        entry: Rc<RefCell<SubmitEntry>>,
        file: File,
        base_offset: u64,
        data: Vec<u8>,
        session_id: u32,
    ) {
        self.fetch_task.push(
            self.fetch_service.fetch_binary(
                Request::post(&url::append_ncch(&format!("{}", session_id)))
                    .body(Ok(data))
                    .unwrap(),
                self.link.send_back(move |response: Response<_>| {
                    let Json(body) = response.into_body();
                    process_response(body, entry.clone(), file.clone(), base_offset)
                }),
            ),
        )
    }

    fn send_ncch_first(
        &mut self,
        entry: Rc<RefCell<SubmitEntry>>,
        file: File,
        base_offset: u64,
        data: Vec<u8>,
    ) {
        self.fetch_task.push(self.fetch_service.fetch_binary(
            Request::post(url::post_ncch()).body(Ok(data)).unwrap(),
            self.link.send_back(move |response: Response<_>| {
                let Json(body) = response.into_body();
                process_response(body, entry.clone(), file.clone(), base_offset)
            }),
        ))
    }

    fn process_ncsd(&mut self, file: File) {
        self.reader_task.push(read_file_ex(
            file.slice(0x120..0x160),
            self.link.send_back(move |data: Vec<u8>| {
                if data.len() != 0x40 {
                    Msg::AddFailedSubmit(file.name())
                } else {
                    Msg::NcsdToNcch(file.clone(), data)
                }
            }),
        ))
    }

    fn ncsd_to_ncch(&mut self, file: File, partition_table: Vec<u8>) {
        for (i, partition) in partition_table.chunks(8).enumerate() {
            let mut temp = [0; 4];
            temp[..].copy_from_slice(&partition[0..4]);
            let offset = u32::from_le_bytes(temp) as u64 * 0x200;
            if offset != 0 {
                self.process_ncch(PARTITION_NAMES[i], file.clone(), offset);
            }
        }
    }

    fn process_file(&mut self, file: File) {
        self.reader_task.push(read_file_ex(
            file.slice(0x100..0x104),
            self.link.send_back(move |data: Vec<u8>| {
                if data == b"NCSD" {
                    Msg::StartProcessNcsd(file.clone())
                } else if data == b"NCCH" {
                    Msg::StartProcessNcch(file.clone(), 0)
                } else {
                    Msg::AddFailedSubmit(file.name())
                }
            }),
        ))
    }
}

impl Component for PageSubmitNcch {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        PageSubmitNcch {
            link,
            submits: vec![],
            reader_task: vec![],
            fetch_service: FetchService::new(),
            fetch_task: vec![],
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Files(files) => {
                for file in files {
                    self.process_file(file);
                }
            }
            Msg::AddFailedSubmit(name) => {
                self.submits.push(Rc::new(RefCell::new(SubmitEntry {
                    name,
                    status: SubmitStatus::FormatError,
                })));
            }
            Msg::StartProcessNcch(file, offset) => {
                self.process_ncch("Standalone NCCH", file, offset);
            }
            Msg::StartProcessNcsd(file) => {
                self.process_ncsd(file);
            }
            Msg::SendNcchFirst(entry, file, offset, data) => {
                self.send_ncch_first(entry, file, offset, data);
            }
            Msg::ProcessMoreNcch(entry, file, base_offset, session_id, offset, len) => {
                self.process_more_ncch(entry, file, base_offset, session_id, offset, len);
            }
            Msg::SendMoreNcch(entry, file, offset, data, session_id) => {
                self.send_more_ncch(entry, file, offset, data, session_id);
            }
            Msg::NcsdToNcch(file, data) => {
                self.ncsd_to_ncch(file, data);
            }
            Msg::None => {}
        }
        true
    }
}

impl PageSubmitNcch {
    fn view_submit_entry(&self, entry: &SubmitEntry) -> Html<Self> {
        let name = match &entry.status {
            SubmitStatus::Succeeded(id) | SubmitStatus::Conflict(id) => html! {
                <a href=format!("{}?{}", url::ncch(), id)>{&entry.name}</a>
            },
            _ => html! { <div class="is-family-monospace">{&entry.name}</div> },
        };

        let status = match entry.status {
            SubmitStatus::Submitting => html! {<span class="tag is-light">{"Processing"}</span>},
            SubmitStatus::Succeeded(_) => html! {<span class="tag is-success">{"Accepted"}</span>},
            SubmitStatus::Conflict(_) => {
                html! {<span class="tag is-warning"><abbr title="The same file is already in the database.">{"Matched"}</abbr></span>}
            }
            SubmitStatus::Busy => {
                html! {<span class="tag is-danger"><abbr title="The server is overloaded.">{"Error"}</abbr></span>}
            }
            SubmitStatus::VerificationFailed => {
                html! {<span class="tag is-danger"><abbr title="The file is tampered or unofficial.">{"Rejected"}</abbr></span>}
            }
            SubmitStatus::ServerError => {
                html! {<span class="tag is-danger"><abbr title="Unexpected server error.">{"Rejected"}</abbr></span>}
            }
            SubmitStatus::FormatError => {
                html! {<span class="tag is-danger"><abbr title="Unsupported format or corrupted file.">{"Rejected"}</abbr></span>}
            }
        };

        html! {
            <tr>
                <td> {status} </td>
                <td class="is-family-monospace"> {name} </td>
            </tr>
        }
    }
}

impl Renderable<PageSubmitNcch> for PageSubmitNcch {
    fn view(&self) -> Html<Self> {
        html! {
            <div class="tile is-ancestor">
                <div class="tile is-vertical is-parent">
                    <div class = "tile is-child">
                        <div class="file is-boxed is-primary"><label class="file-label">
                            <input class="file-input" type="file" accept=".3ds,.app,.cxi,.cci"
                                multiple=true onchange=|value| {
                                let mut result = Vec::new();
                                if let ChangeData::Files(files) = value {
                                    result.extend(files);
                                    Msg::Files(result)
                                } else {
                                    Msg::None
                                }
                            }/>
                            <span class="file-cta">
                                <span class="file-icon">
                                    <i class="fas fa-upload"></i>
                                </span>
                                <span class="file-label">
                                    {"Choose files…"}
                                </span>
                            </span>
                        </label></div>
                        <table class="table is-striped is-narrow is-hoverable">
                            <thead>
                                <tr>
                                    <th>{"Status"}</th>
                                    <th>{"File Name"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for self.submits.iter().map(|e|{
                                    self.view_submit_entry(&e.borrow())
                                })}
                            </tbody>
                        </table>
                    </div>
                </div>
                <div class="tile is-vertical is-parent">
                    <article class="tile is-child">
                        <p class="title">{"Adding Game Entries"}</p>
                        <div class="content">
        {"Two file formats are currently supported: NCCH and NCSD. CIA format is not supported."}<br /><br />
        {"NCCH (*.app, *.cxi) files are typically from digital games, system apps, update patches and DLCs.
        Some common ways to dump them are"}
        <ul>
            <li>
                {"Using "} <a href="https://github.com/d0k3/GodMode9">{"GodMode9"}</a>{": navigate to "}
                <span class="is-family-monospace">{"SYSNAND CTRNAND/title"}</span>
                {" (for system apps), or "}
                <span class="is-family-monospace">{"SYSNAND SD/title"}</span>
                {" (for digital games, updates and DLCs), and for all .app files in the sub folders,
                press A and choose \"Copy to 0:/gm9/out\". Then you can upload these .app files in SD/gm9/out here."}
            </li>
            <li>
                {"Decrypt SD/NAND files without 3DS: mount SD or NAND image on computer using tools such as "}
                <a href="https://github.com/ihaveamac/ninfs">{"ninfs"}</a>
                {" and upload .app files from the mounted file systems."}
            </li>
        </ul>
        <br />
        {"NCSD (*.3ds, *.cci) files are typically from game cartidges, which is a wrapper of multiple NCCH files.
        You can get them simply by dumping the raw content of cartridges. In GodMode9, this is done by
        navigating to "}
        <span class="is-family-monospace">{"GAMECART/<some-id>.3ds"}</span>
        {" and choosing \"Copy to 0:/gm9/out\". You can then upload the .3ds file in SD/gm9/out."}
        <br /><br />
        {"Index3ds only accepts games and contents signed by official, and will reject files that has a wrong
            signature. This means that if the file has ever been modified, it will likely be rejected by index3ds
        (if it is accepted, it means the modified part doesn't affect the information stored in the index3ds database)"}
        <br /><br />
        {"You could, but doesn't need to, decrypt the NCCH/NCSD file before uploading.
        In fact, it is recommended not to decrypt the NCCH/NCSD file, because common decryption tools including
        GodMode9
        would modify the crypto flag upon decryption, and index3ds would need to work extra hard to restore the
        original
        flag in order to verify the content, which doesn't always work well. Note that this should not be confused
        with
        decrypting SD/NAND files mentioned above for NCCH, which is another crypto layer on top of the NCCH
        encryption,
        and which must be performed by users using GodMode9 or ninfs."}
                        </div>
                    </article>
                </div>
            </div>
        }
    }
}
