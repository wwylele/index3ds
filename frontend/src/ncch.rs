use index3ds_common::*;
use yew::format::{json::Json, Nothing};
use yew::prelude::*;
use yew::services::fetch::*;

use crate::language_map;

pub enum Msg {
    NcchInfoReceived(NcchInfo),
    NcchInfoError,
}

#[derive(PartialEq, Properties)]
pub struct PageNcchProp {
    #[props(required)]
    pub ncch_id: String,
}

enum NcchInfoStatus {
    Receiving,
    Error,
    Ready(NcchInfo),
}

pub struct PageNcch {
    props: PageNcchProp,
    ncch_info: NcchInfoStatus,
    fetch_service: FetchService,
    fetch: FetchTask,
}

impl Component for PageNcch {
    type Message = Msg;
    type Properties = PageNcchProp;

    fn create(props: Self::Properties, mut link: ComponentLink<Self>) -> Self {
        let mut fetch_service = FetchService::new();
        let get_request = Request::get(&url::ncch_info(&props.ncch_id, "info"))
            .body(Nothing)
            .unwrap();
        let fetch = fetch_service.fetch(
            get_request,
            link.send_back(|response: Response<_>| {
                let Json(body) = response.into_body();
                match body {
                    Ok(NcchInfoResponse::Ok(ncch_info)) => Msg::NcchInfoReceived(ncch_info),
                    _ => Msg::NcchInfoError,
                }
            }),
        );

        PageNcch {
            props,
            ncch_info: NcchInfoStatus::Receiving,
            fetch_service,
            fetch,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::NcchInfoReceived(info) => self.ncch_info = NcchInfoStatus::Ready(info),
            Msg::NcchInfoError => self.ncch_info = NcchInfoStatus::Error,
        }
        true
    }
}

const LANGUAGE_NAME: &[&str] = &[
    "Japanese",
    "English",
    "French",
    "German",
    "Italian",
    "Spanish",
    "Chinese(S)",
    "Korean",
    "Dutch",
    "Portuguese",
    "Russian",
    "Chinese(T)",
];

const REGION_NAME: &[&str] = &["JPN", "USA", "EUR", "AUS", "CHN", "KOR", "TWN"];

const RATING_NAME: &[&str] = &[
    "CERO",
    "ESRB",
    "?",
    "USK",
    "PEGI GEN",
    "?",
    "PEGI PRT",
    "PEGI BBFC",
    "COB",
    "GRB",
    "CGSRR",
];

const MEMORY_MODE: &[&str] = &[
    "Mode 0",
    "???",
    "Mode 2",
    "Mode 3",
    "Mode 4",
    "Mode 5",
    "N3DS Mode 6",
    "N3DS Mode 7",
];

const MEMORY_TYPE: &[&str] = &["???", "APP", "SYSTEM", "BASE"];

impl PageNcch {
    fn field<T: Into<Html<Self>>>(label: &str, value: T) -> Html<Self> {
        html! {
            <tr>
                <th>
                    {label}
                </th>
                <td class="is-family-monospace">
                    {value.into()}
                </td>
            </tr>
        }
    }

    fn crypto_info(ncch: &NcchInfo) -> Html<Self> {
        if ncch.no_crypto {
            return html! {
                <span class="tag is-danger">{"Plain"}</span>
            };
        }

        if ncch.fixed_key {
            return html! {
                <span class="tag is-warning">{"Zero"}</span>
            };
        }
        let key_type = match ncch.secondary_key_slot {
            0 => "Sec1",
            1 => "Sec2",
            10 => "Sec3",
            11 => "Sec4",
            _ => "???",
        };
        html! {
            <div class="tags">
                <span class="tag is-primary">{key_type}</span>
                {if ncch.seed_crypto {
                    html! {<span class="tag is-info">{"Seed"}</span>}
                } else {
                    html! {}
                }}
            </div>
        }
    }

    fn content_type_info(ncch: &NcchInfo) -> Html<Self> {
        let category = match ncch.content_category {
            0 => "Application",
            1 => "System Update",
            2 => "Manual",
            3 => "DLP Child",
            4 => "Trial",
            _ => "???",
        };
        html! {
            <div class="tags">
                <span class="tag is-primary">{category}</span>
                {if ncch.content_is_data {
                    html! {<span class="tag is-info">{"Data"}</span>}
                } else {
                    html! {}
                }}
                {if ncch.content_is_executable {
                    html! {<span class="tag is-info">{"Executable"}</span>}
                } else {
                    html! {}
                }}
                {if ncch.no_romfs {
                    html! {<span class="tag is-warning">{"No RomFS"}</span>}
                } else {
                    html! {}
                }}
            </div>
        }
    }

    fn region_tags(value: u32) -> Html<Self> {
        html! {
            <div class="tags">
                {for (0..REGION_NAME.len()).map(|i|{
                    if value & (1 << i) != 0 {
                        html!{<span class="tag is-info">{REGION_NAME[i]}</span>}
                    } else {
                        html!{}
                    }
                })}
            </div>
        }
    }

    fn smdh_flag_tags(value: Option<u32>) -> Html<Self> {
        let value = if let Some(value) = value {
            value
        } else {
            return html! {<div>{"N/A"}</div>};
        };
        html! {
            <div class="tags">
                { if value & 0x0001 == 0 {html!{<span class="tag is-warning">{"Invisible"}</span>}} else {html!{}} }
                { if value & 0x0002 != 0 {html!{<span class="tag is-warning">{"Auto-boot"}</span>}} else {html!{}} }
                { if value & 0x0004 != 0 {html!{<span class="tag is-info">{"Allow 3D"}</span>}} else {html!{}} }
                { if value & 0x0008 != 0 {html!{<span class="tag is-info">{"EULA required"}</span>}} else {html!{}} }
                { if value & 0x0010 != 0 {html!{<span class="tag is-info">{"Autosave"}</span>}} else {html!{}} }
                { if value & 0x0020 != 0 {html!{<span class="tag is-info">{"Ex banner"}</span>}} else {html!{}} }
                { if value & 0x0040 != 0 {html!{<span class="tag is-info">{"Rating required"}</span>}} else {html!{}} }
                { if value & 0x0080 != 0 {html!{<span class="tag is-info">{"Use save data"}</span>}} else {html!{}} }
                { if value & 0x0100 == 0 {html!{<span class="tag is-warning">{"No trace"}</span>}} else {html!{}} }
                { if value & 0x0400 != 0 {html!{<span class="tag is-warning">{"Disable backup"}</span>}} else {html!{}} }
                { if value & 0x1000 != 0 {html!{<span class="tag is-warning">{"New 3DS exclusive"}</span>}} else {html!{}} }
            </div>
        }
    }

    fn fs_tags(value: u64) -> Html<Self> {
        let why = html! {
            <>
                { if value & 0x00200000 != 0 {html!{<span class="tag is-warning">{"SeedDB"}</span>}} else {html!{}} }
                { if value & 0x0100000000000000 != 0 {html!{<span class="tag is-warning">{"No RomFS"}</span>}} else {html!{}} }
                { if value & 0x0200000000000000 != 0 {html!{<span class="tag is-warning">{"Extended save data access"}</span>}} else {html!{}} }
            </>
        };
        html! {
            <div class="tags">
                { if value & 0x00000001 != 0 {html!{<span class="tag is-warning">{"System App"}</span>}} else {html!{}} }
                { if value & 0x00000002 != 0 {html!{<span class="tag is-warning">{"Hardware Check"}</span>}} else {html!{}} }
                { if value & 0x00000004 != 0 {html!{<span class="tag is-warning">{"File System Tool"}</span>}} else {html!{}} }
                { if value & 0x00000008 != 0 {html!{<span class="tag is-warning">{"Debug"}</span>}} else {html!{}} }
                { if value & 0x00000010 != 0 {html!{<span class="tag is-warning">{"TWL card backup"}</span>}} else {html!{}} }
                { if value & 0x00000020 != 0 {html!{<span class="tag is-warning">{"TWN NAND data"}</span>}} else {html!{}} }
                { if value & 0x00000040 != 0 {html!{<span class="tag is-warning">{"SpotPass"}</span>}} else {html!{}} }
                { if value & 0x00000080 != 0 {html!{<span class="tag is-info">{"SD"}</span>}} else {html!{}} }
                { if value & 0x00000100 != 0 {html!{<span class="tag is-warning">{"Core"}</span>}} else {html!{}} }
                { if value & 0x00000200 != 0 {html!{<span class="tag is-warning">{"nand:/ro/"}</span>}} else {html!{}} }
                { if value & 0x00000400 != 0 {html!{<span class="tag is-warning">{"nand:/rw/"}</span>}} else {html!{}} }
                { if value & 0x00000800 != 0 {html!{<span class="tag is-warning">{"nand:/ro/ write"}</span>}} else {html!{}} }
                { if value & 0x00001000 != 0 {html!{<span class="tag is-warning">{"System Settings"}</span>}} else {html!{}} }
                { if value & 0x00002000 != 0 {html!{<span class="tag is-warning">{"Cardboard"}</span>}} else {html!{}} }
                { if value & 0x00004000 != 0 {html!{<span class="tag is-warning">{"Export/Import IVS"}</span>}} else {html!{}} }
                { if value & 0x00008000 != 0 {html!{<span class="tag is-info">{"SD write-only"}</span>}} else {html!{}} }
                { if value & 0x00010000 != 0 {html!{<span class="tag is-warning">{"Switch cleanup"}</span>}} else {html!{}} }
                { if value & 0x00020000 != 0 {html!{<span class="tag is-warning">{"Save data move"}</span>}} else {html!{}} }
                { if value & 0x00040000 != 0 {html!{<span class="tag is-warning">{"Shop"}</span>}} else {html!{}} }
                { if value & 0x00080000 != 0 {html!{<span class="tag is-warning">{"Shell"}</span>}} else {html!{}} }
                { if value & 0x00100000 != 0 {html!{<span class="tag is-warning">{"Home menu"}</span>}} else {html!{}} }
                { why }
            </div>
        }
    }

    fn arm9_flag_tags(value: u32) -> Html<Self> {
        html! {
            <div class="tags">
                { if value & 0x00000001 != 0 {html!{<span class="tag is-warning">{"nand:/"}</span>}} else {html!{}} }
                { if value & 0x00000002 != 0 {html!{<span class="tag is-warning">{"nand:/ro/"}</span>}} else {html!{}} }
                { if value & 0x00000004 != 0 {html!{<span class="tag is-warning">{"rwln:/"}</span>}} else {html!{}} }
                { if value & 0x00000008 != 0 {html!{<span class="tag is-warning">{"wnand:/"}</span>}} else {html!{}} }
                { if value & 0x00000010 != 0 {html!{<span class="tag is-warning">{"Card SPI"}</span>}} else {html!{}} }
                { if value & 0x00000020 != 0 {html!{<span class="tag is-warning">{"Use SDIF3"}</span>}} else {html!{}} }
                { if value & 0x00000040 != 0 {html!{<span class="tag is-warning">{"Create seed"}</span>}} else {html!{}} }
                { if value & 0x00000080 != 0 {html!{<span class="tag is-warning">{"Use card SPI"}</span>}} else {html!{}} }
                { if value & 0x00000100 != 0 {html!{<span class="tag is-warning">{"SD app"}</span>}} else {html!{}} }
                { if value & 0x00000200 != 0 {html!{<span class="tag is-warning">{"sdmc:/"}</span>}} else {html!{}} }
            </div>
        }
    }

    fn rating_tags(value: &Option<Vec<u8>>) -> Html<Self> {
        let value = if let Some(value) = value {
            value
        } else {
            return html! {<div>{"N/A"}</div>};
        };
        html! {
            <div class="field is-grouped is-grouped-multiline">
                {for (0..RATING_NAME.len()).map(|i|{
                    if value[i] & 0x80 == 0 {
                        html!{}
                    } else {
                        let (rating, class) = if value[i] & 0x40 != 0 {
                            ("Pending".to_owned(), "is-warning")
                        } else if value[i] & 0x20 != 0 {
                            ("No restriction".to_owned(), "is-info")
                        } else {
                            (format!("{}", value[i] - 0x80), "is-info")
                        };
                        html!{<div class="control"><div class="tags has-addons">
                            <span class="tag is-dark">{RATING_NAME[i]}</span>
                            <span class=format!("tag {}", class)>{rating}</span>
                        </div></div>}
                    }
                })}
            </div>
        }
    }

    fn core_flag_tags(ncch: &NcchInfo) -> Html<Self> {
        html! {
            <div class="tags">
                { if ncch.high_cpu_speed == Some(false) && ncch.enable_l2_cache == Some(false) {
                    html!{<span class="tag is-dark">{"Legacy mode"}</span>}
                } else {
                    html!{}
                }}
                { if ncch.high_cpu_speed.unwrap_or(false) {html!{<span class="tag is-info">{"High speed"}</span>}} else {html!{}} }
                { if ncch.enable_l2_cache.unwrap_or(false) {html!{<span class="tag is-info">{"L2 cache"}</span>}} else {html!{}} }
            </div>
        }
    }

    fn affinity_tags(affinity: u8) -> Html<Self> {
        html! {
            <div class="tags">
                { for (0..4).map(|i| if affinity & (1 << i) != 0 {
                    html!{<span class="tag is-info">{format!("{}", i)}</span>}
                } else {
                    html!{}
                })}
            </div>
        }
    }

    fn memory_tags(ncch: &NcchInfo) -> Html<Self> {
        if ncch.system_mode.is_none() {
            return html! {};
        }
        let o3ds_mode = MEMORY_MODE[ncch.system_mode.unwrap() as usize];
        let o3ds_class = if ncch.system_mode.unwrap() == 0 {
            "is-info"
        } else {
            "is-warning"
        };
        let (n3ds_mode, n3ds_class) = match ncch.n3ds_system_mode.unwrap() {
            0 => ("N3DS compat mode", "is-dark"),
            2 => (MEMORY_MODE[7], "is-warning"),
            _ => (MEMORY_MODE[6], "is-info"),
        };
        html! {
            <div class="tags">
                <span class=format!("tag {}", o3ds_class)>{o3ds_mode}</span>
                <span class=format!("tag {}", n3ds_class)>{n3ds_mode}</span>
            </div>
        }
    }

    #[allow(clippy::if_same_then_else)]
    fn kernel_tags(desc: &[u32]) -> Html<Self> {
        let mut tags = Vec::<Html<Self>>::new();
        for d in desc {
            if d & 0b11110000_00000000_00000000_00000000 == 0b11100000_00000000_00000000_00000000 {
                /*tags.push(html! {
                    <div class="tags has-addons">
                        <span class="tag is-dark">{"Interrupt"}</span>
                        <span class="tag is-warning">{format!("{:08x}", d)}</span>
                    </div>
                });*/
            } else if d & 0b11111000_00000000_00000000_00000000
                == 0b11110000_00000000_00000000_00000000
            {
                /*for i in 0..24u32 {
                    if d & (1 << i) != 0 {
                        tags.push(html! {
                            <div class="tags has-addons">
                                <span class="tag is-dark">{"SVC"}</span>
                                <span class="tag is-info">{format!("{}", i + ((d >> 24) & 7u32) * 24u32)}</span>
                            </div>
                        });
                    }
                }*/
            } else if d & 0b11111110_00000000_00000000_00000000
                == 0b11111100_00000000_00000000_00000000
            {
                tags.push(html! {
                    <div class="tags has-addons">
                        <span class="tag is-dark">{"Version"}</span>
                        <span class="tag is-primary">{format!("{}.{}", (d >> 8) & 0xFFu32, d & 0xFFu32)}</span>
                    </div>
                });
            } else if d & 0b11111111_00000000_00000000_00000000
                == 0b11111110_00000000_00000000_00000000
            {
                tags.push(html! {
                    <div class="tags has-addons">
                        <span class="tag is-dark">{"Handle table size"}</span>
                        <span class="tag is-info">{format!("{}", d & 0x7FF)}</span>
                    </div>
                });
            } else if d & 0b11111111_10000000_00000000_00000000
                == 0b11111111_00000000_00000000_00000000
            {
                if d & 0x1 != 0 {
                    tags.push(html! {<span class="tag is-info">{"Allow debugging"}</span>});
                }
                if d & 0x2 != 0 {
                    tags.push(html! {<span class="tag is-info">{"Force debugging"}</span>});
                }
                if d & 0x4 != 0 {
                    tags.push(html! {<span class="tag is-info">{"Allow non-alphanum"}</span>});
                }
                if d & 0x8 != 0 {
                    tags.push(html! {<span class="tag is-warning">{"Shared page writing"}</span>});
                }
                if d & 0x10 != 0 {
                    tags.push(html! {<span class="tag is-warning">{"Privilege priority"}</span>});
                }
                if d & 0x20 != 0 {
                    tags.push(html! {<span class="tag is-warning">{"main() args"}</span>});
                }
                if d & 0x40 != 0 {
                    tags.push(html! {<span class="tag is-warning">{"Shared device memory"}</span>});
                }
                if d & 0x80 != 0 {
                    tags.push(html! {<span class="tag is-warning">{"Runnable on sleep"}</span>});
                }
                if d & 0x200 != 0 {
                    tags.push(html! {<span class="tag is-warning">{"Special memory"}</span>});
                }
                if d & 0x400 != 0 {
                    tags.push(html! {<span class="tag is-warning">{"Core 2"}</span>});
                }
                tags.push(html! {
                    <div class="tags has-addons">
                        <span class="tag is-dark">{"Memory type"}</span>
                        <span class="tag is-info">{MEMORY_TYPE.get(((d >> 8) & 0xFu32) as usize)
                            .cloned().unwrap_or("???")}</span>
                    </div>
                });
            } else if d & 0b11111111_11100000_00000000_00000000
                == 0b11111111_10000000_00000000_00000000
            {
                /*tags.push(html! {
                    <div class="tags has-addons">
                        <span class="tag is-dark">{"Map Range"}</span>
                        <span class="tag is-info">{
                            format!("{:08x}{}", (d & 0xFFFFF) << 12,
                            if d & (1 << 20) != 0 {"ro"} else {"rw"})}</span>
                    </div>
                });*/
            } else if d & 0b11111111_11100000_00000000_00000000
                == 0b11111111_11100000_00000000_00000000
            {
                /*tags.push(html! {
                    <div class="tags has-addons">
                        <span class="tag is-dark">{"Map"}</span>
                        <span class="tag is-info">{
                            format!("{:08x}{}", (d & 0xFFFFF) << 12,
                            if d & (1 << 20) != 0 {"ro"} else {"rw"})}</span>
                    </div>
                });*/
            } else {
                tags.push(html! {
                    <div class="tags has-addons">
                        <span class="tag is-dark">{"Unknown"}</span>
                        <span class="tag is-danger">{format!("{:08x}", d)}</span>
                    </div>
                });
            }
        }
        html! {
            <div class="field is-grouped is-grouped-multiline">
            {
                for tags.into_iter().map(|v|html!{ <div class="control">{v}</div> })
            }
            </div>
        }
    }

    fn titles(ncch: &NcchInfo) -> Html<Self> {
        let mut language_set = std::collections::HashSet::new();
        for (i, languages) in language_map::LANGUAGE_MAP.iter().enumerate() {
            if ncch.region_lockout.unwrap_or(0) & (1 << i) != 0 {
                for l in *languages {
                    language_set.insert(l);
                }
            }
        }

        let with_line_break = |s: &str| -> Html<Self> {
            html! {
                for s.split('\n').map(|l|html!{
                    <>
                        {l}<br/>
                    </>
                })
            }
        };

        html! {
            <tbody>
            {for (0..12).map(|i|{
                let long = ncch.long_title.as_ref().map(|v|v[i].as_str()).unwrap_or("");
                let short = ncch.short_title.as_ref().map(|v|v[i].as_str()).unwrap_or("");
                let publisher = ncch.publisher.as_ref().map(|v|v[i].as_str()).unwrap_or("");
                let disable_class = if language_set.contains(&i) {""} else {"has-text-grey"};
                html! {
                    <tr class=disable_class>
                        <td>{LANGUAGE_NAME[i]}</td>
                        <td>{with_line_break(long)}</td>
                        <td>{with_line_break(short)}</td>
                        <td>{with_line_break(publisher)}</td>
                    </tr>
                }
            })}
            </tbody>
        }
    }
}

const CONTENT_SIZE_UNIT: &[&str] = &["GiB", "MiB", "KiB"];

fn format_content_size(size: u64) -> String {
    let mut thres = 1024u64.pow(CONTENT_SIZE_UNIT.len() as u32);
    for unit in CONTENT_SIZE_UNIT {
        if size >= thres {
            return format!(
                "{:.2} {} ({} bytes)",
                size as f32 / thres as f32,
                unit,
                size
            );
        }
        thres /= 1024;
    }
    format!("{} Bytes", size)
}

fn format_platform(value: u8) -> String {
    match value {
        1 => "3DS".to_owned(),
        2 => "New 3DS".to_owned(),
        _ => format!("???{}", value),
    }
}

fn accessible_save(ncch: &NcchInfo) -> String {
    if ncch.storage_access_id.is_none() {
        return "".to_owned();
    }
    let saves = ncch.storage_access_id.clone().unwrap();
    let mut result = format!("{} {} {}", &saves[11..16], &saves[6..11], &saves[1..6]);
    if ncch.filesystem_flag.unwrap() & 0x02000000_00000000 != 0 {
        let saves2 = ncch.extdata_id.clone().unwrap();
        result = format!(
            "{} {} {} {}",
            result,
            &saves2[11..16],
            &saves2[6..11],
            &saves2[1..6]
        )
    }
    result
}

fn accessible_extdata(ncch: &NcchInfo) -> String {
    if ncch.extdata_id.is_none() {
        return "".to_owned();
    }
    if ncch.filesystem_flag.unwrap() & 0x02000000_00000000 != 0 {
        return "".to_owned();
    }
    ncch.extdata_id.clone().unwrap()
}

const RESOURCE_LIMIT_CATEGORY: &[&str] =
    &["Application", "System applet", "Library applet", "Other"];

fn resource_limit(value: Option<u8>) -> &'static str {
    if value.is_none() {
        return "";
    }
    RESOURCE_LIMIT_CATEGORY
        .get(value.unwrap() as usize)
        .cloned()
        .unwrap_or("???")
}

impl Renderable<PageNcch> for PageNcch {
    fn view(&self) -> Html<Self> {
        match &self.ncch_info {
            NcchInfoStatus::Receiving => html! {"Receiving"},
            NcchInfoStatus::Error => html! {"Error"},
            NcchInfoStatus::Ready(ncch_info) => {
                let unit_size = 0x200 * (1 << ncch_info.content_unit_size);
                let (icon_large, icon_small) = if ncch_info.short_title.is_some() {
                    (
                        url::ncch_info(&self.props.ncch_id, "icon_large.png"),
                        url::ncch_info(&self.props.ncch_id, "icon_small.png"),
                    )
                } else {
                    (
                        url::not_found_large().to_owned(),
                        url::not_found_small().to_owned(),
                    )
                };

                html! {
                    <div class="tile is-ancestor">
                        <div class="tile is-parent is-vertical">
                            <div class="tile is-child">
                                <p class="title">{"Game Icon"}</p>
                                <table class="table"><tbody>
                                    <tr>
                                        <td>{"Large"}</td>
                                        <td><img src=icon_large/></td>
                                    </tr>
                                    <tr>
                                        <td>{"Small"}</td>
                                        <td><img src=icon_small/></td>
                                    </tr>
                                </tbody></table>
                            </div>

                            <div class="tile is-child">
                                <p class="title">{"Partition Information"}</p>
                                <table class="table"><tbody>
                                    {PageNcch::field("Partition ID", &ncch_info.partition_id)}
                                    {PageNcch::field("Program ID", &ncch_info.program_id)}
                                    {PageNcch::field("Product Code", &ncch_info.product_code)}
                                    {PageNcch::field("Maker Code", &ncch_info.maker_code)}
                                    {PageNcch::field("Content Type", PageNcch::content_type_info(&ncch_info))}
                                    {PageNcch::field("Content Size", &format_content_size(ncch_info.content_size as u64 * unit_size))}
                                    {PageNcch::field("NCCH Version", &format!("{}", ncch_info.ncch_verson))}
                                    {PageNcch::field("Crypto Type", PageNcch::crypto_info(&ncch_info))}
                                    {PageNcch::field("Platform", &format_platform(ncch_info.platform))}
                                </tbody></table>
                            </div>
                            <div class="tile is-child">
                                <p class="title">{"Game Title"}</p>
                                <table class="table">
                                    <thead>
                                        <th>{"Language"}</th>
                                        <th>{"Long Title"}</th>
                                        <th>{"Short Title"}</th>
                                        <th>{"Publisher"}</th>
                                    </thead>
                                    {PageNcch::titles(ncch_info)}
                                </table>
                            </div>
                            <div class="tile is-child">
                                <p class="title">{"Home Menu Interaction"}</p>
                                <table class="table"><tbody>
                                    {PageNcch::field("Match Maker ID", &format!("{}-{}",
                                        ncch_info.match_maker_id.as_ref().map(|x|&**x).unwrap_or(""),
                                        ncch_info.match_maker_bit_id.as_ref().map(|x|&**x).unwrap_or("")))}
                                    {PageNcch::field("StreetPass ID", ncch_info.cec_id.as_ref().map(|x|&**x).unwrap_or(""))}
                                    {PageNcch::field("EULA Version", &ncch_info.eula_version.map(
                                        |x|format!("{}.{}", x / 256, x %256)).unwrap_or_default())}
                                    {PageNcch::field("Regions", PageNcch::region_tags(ncch_info.region_lockout.unwrap_or_default()))}
                                    {PageNcch::field("Ratings", PageNcch::rating_tags(&ncch_info.ratings))}
                                    {PageNcch::field("Flags", PageNcch::smdh_flag_tags(ncch_info.smdh_flags))}
                                </tbody></table>
                            </div>
                        </div>
                        <div class="tile is-parent is-vertical">
                            <div class="tile is-child">
                                <p class="title">{"System & Access Control"}</p>
                                <table class="table"><tbody>
                                    {PageNcch::field("Title ID", ncch_info.exheader_program_id.as_ref().map(|x|&**x).unwrap_or(""))}
                                    {PageNcch::field("Jump ID", ncch_info.jump_id.as_ref().map(|x|&**x).unwrap_or(""))}
                                    {PageNcch::field("Process Name", ncch_info.exheader_name.as_ref().map(|x|&**x).unwrap_or(""))}
                                    {PageNcch::field("SD App", ncch_info.sd_app.map(|v|format!("{}", v)).unwrap_or_default())}
                                    {PageNcch::field("Remaster Version", ncch_info.remaster_version.map(|v|format!("{}", v)).unwrap_or_default())}
                                    {PageNcch::field("Save Data Size", ncch_info.save_data_size.map(|v|format_content_size(v)).unwrap_or_default())}
                                    {PageNcch::field("Firmware ID", ncch_info.core_version.map(|v|format!("{}", v)).unwrap_or_default())}
                                    {PageNcch::field("N3DS CPU", PageNcch::core_flag_tags(&ncch_info))}
                                    {PageNcch::field("Memory Mode", PageNcch::memory_tags(&ncch_info))}
                                    {PageNcch::field("Thread Priority", ncch_info.thread_priority.map(|v|format!("{}", v)).unwrap_or_default())}
                                    {PageNcch::field("Ideal Processor", ncch_info.ideal_processor.map(|v|format!("{}", v)).unwrap_or_default())}
                                    {PageNcch::field("Affinity Mask", PageNcch::affinity_tags(ncch_info.affinity_mask.unwrap_or_default()))}
                                    {PageNcch::field("File System Permissions", PageNcch::fs_tags(ncch_info.filesystem_flag.unwrap_or_default()))}
                                    {PageNcch::field("System Save IDs", &format!("{} {}",
                                        ncch_info.system_savedata_id0.as_ref().map(|x|&**x).unwrap_or(""),
                                        ncch_info.system_savedata_id1.as_ref().map(|x|&**x).unwrap_or("")))}
                                    {PageNcch::field("Accessible Save IDs", &accessible_save(&ncch_info))}
                                    {PageNcch::field("Extdata ID", &accessible_extdata(&ncch_info))}
                                    {PageNcch::field("Resource Limit",resource_limit(ncch_info.resource_limit_category))}
                                    {PageNcch::field("Core 1 Usage",ncch_info.resource_limit_desc.as_ref().map(
                                        |v|format!("{}", v.get(0).cloned().unwrap_or(0))).unwrap_or_default())}
                                    {PageNcch::field("ARM9 Control Version", ncch_info.arm9_flag_version.map(|v|format!("{}", v)).unwrap_or_default())}
                                    {PageNcch::field("ARM9 Permissions", PageNcch::arm9_flag_tags(ncch_info.arm9_flag.unwrap_or(0)))}
                                    {PageNcch::field("Kernel Capabilities", PageNcch::kernel_tags(ncch_info.kernel_desc.as_ref().map(|x|&**x).unwrap_or(&[])))}
                                </tbody></table>
                            </div>
                        </div>
                    </div>
                }
            }
        }
    }
}
